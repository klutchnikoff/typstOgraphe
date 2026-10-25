//! Un seul calcul pour menus, palette, barre, boutons et dispatch clavier.
//! L’interface garde les états de fenêtres/préférences ; le moteur décide des cibles.
use super::*;
use crate::{
    commands::Route,
    engine::{Availability, Command as Core, TextTarget},
};

impl App {
    pub(super) fn ui_text_target(&self) -> Option<TextTarget> {
        self.document
            .engine
            .selection_target(editor::to_core(self.document.selection))
            .ok()
            .map(Into::into)
    }
    /// Éligibilité hors modalités : la palette évalue les actions de sa propre fenêtre.
    /// Les hôtes de menus et le dispatch externe appliquent en plus modal_open().
    pub(super) fn command_enabled(&self, command: Command) -> bool {
        if let Some(enabled) = self.review_command_enabled(command) {
            return enabled;
        }
        if self.review_open() {
            return false;
        }
        if self.bands.field_focused
            && (matches!(command, Command::Undo | Command::Redo | Command::Clear)
                || self
                    .keymap
                    .registry()
                    .definitions()
                    .any(|d| d.command == command && d.scope == crate::commands::Scope::Editor))
        {
            return false;
        }
        if !self.keymap.registry().contains(command)
            || (!self.document.open && !command.available_without_document())
        {
            return false;
        }
        if command == Command::Clear && self.document.history_field.is_some() {
            return false;
        }
        if matches!(
            command,
            Command::NextSpelling | Command::PreviousSpelling | Command::IgnoreSpelling
        ) && !self.preferences.spelling.enabled
        {
            return false;
        }
        let target = match self.ui_text_target() {
            Some(target) => target,
            None if self.document.open => return false,
            None => TextTarget::default(),
        };
        let enabled = |command| {
            matches!(
                self.document.engine.available(&command),
                Availability::Enabled
            )
        };
        match command.route(target) {
            Route::Document(core) => enabled(core),
            Route::Prepared(intent) => matches!(
                self.document.engine.available_intent(intent, target),
                Availability::Enabled
            ),
            Route::Contextual => self.context_document_command(command).is_some_and(enabled),
            Route::Interface => match command {
                Command::ShareSelection => {
                    !self.range().is_empty()
                        && !self.assistance_busy_without_palette()
                        && self
                            .assistance
                            .session
                            .as_ref()
                            .is_some_and(|s| !s.finished() && s.review().is_none())
                }
                Command::ContextualAction => self.contextual_action().is_some(),
                Command::RemoveWidget => {
                    self.document.ime.draft.is_none()
                        && self.current_widget_removal().is_some_and(enabled)
                }
                Command::RecoverDocuments => !self.recovery.entries.is_empty(),
                Command::SpellingSuggestions | Command::LearnSpelling => {
                    self.current_spelling_reference().is_some()
                }
                _ => true,
            },
            Route::Unsupported => false,
        }
    }
    /// Les mêmes paramètres capturés servent à la disponibilité et à l’exécution.
    pub(super) fn context_document_command(&self, command: Command) -> Option<Core> {
        use crate::engine::{
            FindParams, GuardParams, ReplaceAllParams, ReplaceCurrentParams, SpellingIssueParams,
            TargetParams,
        };
        let target = self.ui_text_target()?;
        let guard = self.document.engine.target_guard();
        Some(match command {
            Command::Model(model) => {
                Core::from_call(model.id(), serde_json::json!({"guard":guard})).ok()?
            }
            Command::FindNext | Command::FindPrevious => {
                let p = FindParams {
                    target,
                    query: self.search.engine.query.clone(),
                    case_sensitive: self.search.engine.case_sensitive,
                };
                if command == Command::FindNext {
                    Core::FindNext(p)
                } else {
                    Core::FindPrevious(p)
                }
            }
            Command::ReplaceAll => Core::ReplaceAll(ReplaceAllParams {
                guard,
                query: self.search.engine.query.clone(),
                case_sensitive: self.search.engine.case_sensitive,
                replacement: self.search.engine.replacement.clone(),
            }),
            Command::ReplaceCurrent => {
                let index = self.search.engine.selected(&self.range())?;
                Core::ReplaceCurrent(ReplaceCurrentParams {
                    occurrence: self.search.occurrences.get(index)?.clone(),
                    query: self.search.engine.query.clone(),
                    case_sensitive: self.search.engine.case_sensitive,
                    replacement: self.search.engine.replacement.clone(),
                })
            }
            Command::NextSpelling => Core::NextSpelling(TargetParams { target }),
            Command::PreviousSpelling => Core::PreviousSpelling(TargetParams { target }),
            Command::IgnoreSpelling => Core::IgnoreSpelling(SpellingIssueParams {
                issue: self.current_spelling_reference()?,
            }),
            Command::ResetSpelling => Core::ResetSpelling(GuardParams { guard }),
            _ => return None,
        })
    }
    /// Le soulignement peut attendre le worker ; la disponibilité lit le document publié.
    pub(super) fn current_spelling_reference(&self) -> Option<crate::engine::SpellingIssue> {
        self.spelling_reference_at(self.range().start)
    }
    pub(super) fn spelling_reference_at(&self, at: usize) -> Option<crate::engine::SpellingIssue> {
        use crate::engine::{Empty, Query, SpellingIssues};
        if !self.preferences.spelling.enabled || !self.document.open {
            return None;
        }
        let issues: SpellingIssues = serde_json::from_value(
            self.document
                .engine
                .query(Query::SpellingIssues(Empty {}))
                .ok()?,
        )
        .ok()?;
        issues.issues.into_iter().find(|issue| {
            self.document
                .engine
                .resolve_target(issue.target)
                .is_ok_and(|(start, end)| {
                    (start..end).contains(&at)
                        || (at == end && (self.range().is_empty() || self.range() == (start..end)))
                })
        })
    }
}
