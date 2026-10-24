//! Découverte sans UI et lectures du document publié. Les capacités décrivent
//! les étapes documentaires du type, indépendamment de la sélection courante.
use super::*;
use crate::document_types::DocumentType;
use serde_json::json;

impl Engine {
    pub(super) fn capabilities(kind: DocumentType) -> Vec<&'static str> {
        Command::command_specs()
            .iter()
            .filter(|spec| {
                spec.batch_step
                    && (spec.availability != AvailabilityRule::Letter
                        || kind == DocumentType::Courrier)
            })
            .map(|spec| spec.id.as_str())
            .collect()
    }

    pub(super) fn document_types_query() -> Value {
        let types: Vec<_> = DocumentType::ALL.into_iter().map(|kind| {
            let models: Vec<_> = kind.models().map(|model| json!({
                "id": model.model.id(),
                "name": model.name,"description":model.description,"keywords":model.keywords,
                "fixed_page": model.page.map(|(paper,landscape)| json!({"paper":paper,"landscape":landscape})),
            })).collect();
            json!({"id":kind,"can_create":kind.can_create(),"models":models,
                "capabilities":Self::capabilities(kind)})
        }).collect();
        json!({"types": types})
    }

    pub(super) fn assets_query(&self) -> Result<Value, CommandError> {
        self.check_rule(AvailabilityRule::Read)?;
        let retained = crate::rendering::retained_assets(
            self.document().source(),
            self.metadata(),
            self.template(),
            &self.document().assets,
            self.dependencies(),
        )
        .map_err(|e| CommandError::with_cause(ErrorCode::Document, "asset_inventory", e))?;
        let inventory = |files: &crate::assets::Assets| -> Vec<Value> {
            files
                .iter()
                .map(|(path, data)| json!({"path":path,"bytes":data.len()}))
                .collect()
        };
        Ok(
            json!({"guard":self.target_guard(),"assets":inventory(&retained),
            "dependencies":inventory(self.dependencies())}),
        )
    }

    pub(super) fn commands_query(&self) -> Value {
        let entries: Vec<_> = catalogue()
            .iter()
            .map(|spec| {
                let mut value = serde_json::to_value(spec).expect("serializable spec");
                value["context_availability"] = self.catalogue_availability(spec);
                value
            })
            .collect();
        json!({"guard":self.open.then(|| self.target_guard()),
            "selection": self.open.then(|| self.selection_target(self.selection).ok()).flatten(),
            "entries":entries})
    }

    fn catalogue_availability(&self, spec: &CommandSpec) -> Value {
        let id = spec.id.as_str();
        if let Err(reason) = self.check_access(id) {
            return serde_json::to_value(Availability::Disabled { reason }).expect("availability");
        }
        // Une commande valide avec ses seuls défauts est réellement vérifiable.
        // Aucun chemin, texte, garde ou ressource artificiel n’est fabriqué.
        if let Ok(command) = Command::from_call(id, json!({})) {
            return serde_json::to_value(self.available(&command)).expect("availability");
        }
        let intent = match id {
            "document.new" => Some(CommandIntent::New),
            "document.open" => Some(CommandIntent::Open),
            "document.save" => Some(CommandIntent::Save),
            "document.save_as" => Some(CommandIntent::SaveAs),
            "document.close" => Some(CommandIntent::Close),
            "document.export" => Some(CommandIntent::Export),
            "text.link" => Some(CommandIntent::Link),
            "insert.image" => Some(CommandIntent::InsertImage),
            "document.set_model" => Some(CommandIntent::SetModel),
            _ => None,
        };
        let availability = if let Some(intent) = intent {
            self.available_intent(intent, TextTarget::default())
        } else if matches!(
            id,
            "document.inspect"
                | "commands.list"
                | "document_types.list"
                | "recovery.list"
                | "spelling.suggestions"
                | "recovery.delete"
                | "recovery.restore"
        ) {
            Availability::Enabled
        } else {
            match self.check_rule(spec.availability) {
                Ok(()) => Availability::Enabled,
                Err(reason) => Availability::Disabled { reason },
            }
        };
        if let Availability::Disabled { .. } = availability {
            return serde_json::to_value(availability).expect("availability");
        }
        // Les paramètres obligatoires et leurs contrôles restent à fournir.
        // L'état n'affirme donc pas qu'une exécution quelconque serait acceptée.
        if spec.params_schema["required"]
            .as_array()
            .is_some_and(|v| !v.is_empty())
        {
            json!({"state":"requires_parameters"})
        } else {
            json!({"state":"enabled"})
        }
    }
}
