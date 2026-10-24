//! Une session privée pour les étapes, une publication et au plus une écriture finale.
use super::*;
use serde_json::json;
use std::io::{self, Write};

pub const MAX_BATCH_STEPS: usize = 256;
pub const MAX_BATCH_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_BATCH_RESOURCE_BYTES: usize = crate::assets::MAX_DOCUMENT_BYTES as usize;

/// Compter sans allouer une deuxième copie du JSON (notamment des images base64).
struct ByteLimit(usize);
impl Write for ByteLimit {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.0 = self
            .0
            .checked_sub(data.len())
            .ok_or_else(|| io::Error::other("batch size limit"))?;
        Ok(data.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Command {
    pub fn batch_step_allowed(&self) -> bool {
        Self::command_specs()
            .iter()
            .find(|spec| spec.id.as_str() == self.id())
            .expect("registered command")
            .batch_step
    }
}
fn step_error(index: usize, id: &str, error: CommandError) -> CommandError {
    CommandError {
        code: error.code,
        params: json!({"reason":"batch_step","step":index,"command":id,"error":error}),
    }
}
impl Engine {
    /// Le contrôle de toutes les étapes précède même les lectures des imports.
    pub(super) fn check_batch(&self, p: &BatchParams) -> Result<Vec<Command>, CommandError> {
        self.check_access("document.batch")?;
        self.check_guard(p.guard)?;
        if !self.open {
            return Err(CommandError::new(ErrorCode::Unavailable, "no_document"));
        }
        if p.steps.is_empty() || p.steps.len() > MAX_BATCH_STEPS {
            return Err(CommandError::new(
                ErrorCode::InvalidParams,
                "batch_step_limit",
            ));
        }
        serde_json::to_writer(ByteLimit(MAX_BATCH_BYTES), p)
            .map_err(|_| CommandError::new(ErrorCode::InvalidParams, "batch_size_limit"))?;
        p.steps
            .iter()
            .enumerate()
            .map(|(index, call)| {
                let command = Command::from_call(call.id.as_str(), call.params.clone())
                    .map_err(|e| step_error(index, call.id.as_str(), e))?;
                if !command.batch_step_allowed() {
                    return Err(step_error(
                        index,
                        command.id(),
                        CommandError::new(ErrorCode::Unavailable, "batch_step_forbidden"),
                    ));
                }
                Ok(command)
            })
            .collect()
    }
    /// Borner aussi les imports supprimés plus tard dans le lot : leurs octets
    /// ne sont pas couverts par la taille du script, et aucun undo interne ne les exige.
    pub(super) fn check_batch_resources(&self, draft: &Engine) -> Result<(), CommandError> {
        let added = |current: &crate::assets::Assets, original: &crate::assets::Assets| {
            current
                .iter()
                .filter(|(path, data)| original.get(*path) != Some(*data))
                .map(|(_, data)| data.len())
                .sum::<usize>()
        };
        let growth = added(
            &draft.session.document.assets,
            &self.session.document.assets,
        ) + added(&draft.session.dependencies, &self.session.dependencies);
        if growth > MAX_BATCH_RESOURCE_BYTES {
            return Err(CommandError::new(
                ErrorCode::Document,
                "batch_resource_limit",
            ));
        }
        Ok(())
    }
    pub(super) fn run_batch(&mut self, p: BatchParams) -> Result<Outcome, CommandError> {
        let commands = self.check_batch(&p)?;
        let mut draft: Engine = self.session.fork_for_batch().into();
        draft.selection = self.selection;
        draft.spelling = self.spelling.fork();
        let mut reveal = false;
        for (index, command) in commands.into_iter().enumerate() {
            let id = command.id();
            let outcome = draft.run(command).map_err(|e| step_error(index, id, e))?;
            self.check_batch_resources(&draft)
                .map_err(|e| step_error(index, id, e))?;
            reveal |= outcome.effects.contains(&Effect::RevealSelection);
            // L’historique provisoire ne sert pas à annuler les étapes individuellement.
            // Ne pas garder 256 copies du document ; seul l’instantané publié sera retenu.
            draft.session.undo.clear();
            draft.session.redo.clear();
            draft.session.typing_group = false;
        }
        self.finish_batch(p, draft, reveal)
    }
    fn finish_batch(
        &mut self,
        p: BatchParams,
        draft: Engine,
        reveal: bool,
    ) -> Result<Outcome, CommandError> {
        let model_changed = self.session.template != draft.session.template
            || self.session.dependencies != draft.session.dependencies;
        let presentation = if model_changed {
            Some(draft.option_policy()?)
        } else {
            None
        };
        let before = self.session.snapshot(self.selection);
        // Le candidat publié conserve la garde de départ ; les révisions internes
        // ne sortent jamais du lot et les cibles externes ne sont jamais réancrées.
        let mut candidate = before.clone();
        candidate.document = draft.session.document;
        candidate.metadata = draft.session.metadata;
        candidate.template = draft.session.template;
        candidate.dependencies = draft.session.dependencies;
        let mut prepared: Engine = self.session.fork_for_batch().into();
        prepared.selection = prepared.session.commit_snapshot(
            p.guard,
            before,
            candidate,
            draft.selection,
            EditGroup::Discrete,
        )?;
        let changed = prepared.session.revision != self.session.revision;
        let mut saved = None;
        if let Some(save) = p.save {
            let guard = prepared.session.target_guard();
            let command = match save {
                BatchSave::Current {} => {
                    Command::SaveDocument(SaveDocumentParams { guard, path: None })
                }
                BatchSave::As { path, overwrite } => Command::SaveAs(SaveAsParams {
                    guard,
                    path,
                    overwrite,
                }),
            };
            let outcome = prepared.run(command)?;
            saved = outcome.value.map(|value| value["path"].clone());
        }
        // Plus d’opération faillible après la sauvegarde : transfert des champs et
        // de l’unique transaction préparée, en conservant l’historique antérieur.
        self.session.adopt_batch(prepared.session);
        self.selection = prepared.selection;
        self.presentation_cache = Default::default();
        if let Some(policy) = &presentation {
            self.cache_presentation(policy.clone());
        }
        Ok(Outcome {
            changed,
            guard: self.session.target_guard(),
            selection: self.session.selection_target(self.selection).ok(),
            effects: [
                changed.then_some(Effect::Render),
                reveal.then_some(Effect::RevealSelection),
                saved.is_some().then_some(Effect::Saved),
            ]
            .into_iter()
            .flatten()
            .collect(),
            value: Some(json!({"steps":p.steps.len(),"saved":saved,"presentation":presentation})),
        })
    }
}
