//! Disponibilité avant la collecte des paramètres par un dialogue : aucune URL,
//! ressource ou destination fictive, aucune lecture ni écriture de fichier.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandIntent {
    New,
    Open,
    Save,
    SaveAs,
    Close,
    Export,
    Link,
    InsertImage,
    SetModel,
}
impl CommandIntent {
    pub fn id(self) -> &'static str {
        match self {
            Self::New => "document.new",
            Self::Open => "document.open",
            Self::Save => "document.save",
            Self::SaveAs => "document.save_as",
            Self::Close => "document.close",
            Self::Export => "document.export",
            Self::Link => "text.link",
            Self::InsertImage => "insert.image",
            Self::SetModel => "document.set_model",
        }
    }
}
impl Engine {
    pub(super) fn check_access(&self, id: &str) -> Result<(), CommandError> {
        if self.is_read_only()
            && catalogue().iter().any(|spec| {
                spec.id.as_str() == id
                    && (matches!(
                        spec.family,
                        Family::Edit
                            | Family::Image
                            | Family::Letter
                            | Family::History
                            | Family::Batch
                    ) || id == "document.save")
            })
        {
            return Err(CommandError::new(ErrorCode::Unavailable, "file_read_only"));
        }
        Ok(())
    }
    pub fn available_intent(&self, intent: CommandIntent, target: TextTarget) -> Availability {
        let checked = || {
            self.check_access(intent.id())?;
            let rule = Command::command_specs()
                .iter()
                .find(|spec| spec.id.as_str() == intent.id())
                .expect("registered intent")
                .availability;
            self.check_rule(rule)?;
            match intent {
                CommandIntent::Link => self.check_link_target(target),
                CommandIntent::InsertImage => self.check_image_insertion(target),
                CommandIntent::Export => self.check_export_fields(),
                _ => Ok(()),
            }
        };
        match checked() {
            Ok(()) => Availability::Enabled,
            Err(reason) => Availability::Disabled { reason },
        }
    }
    pub(super) fn check_rule(&self, rule: AvailabilityRule) -> Result<(), CommandError> {
        if !self.open && rule != AvailabilityRule::Always {
            return Err(CommandError::new(ErrorCode::Unavailable, "no_document"));
        }
        if rule == AvailabilityRule::Letter
            && self.session.metadata.document_type()
                != crate::document_types::DocumentType::Courrier
        {
            return Err(CommandError::new(ErrorCode::Unavailable, "document_type"));
        }
        if (rule == AvailabilityRule::Undo && !self.can_undo())
            || (rule == AvailabilityRule::Redo && !self.can_redo())
        {
            return Err(CommandError::new(
                ErrorCode::Unavailable,
                if rule == AvailabilityRule::Undo {
                    "undo_empty"
                } else {
                    "redo_empty"
                },
            ));
        }
        Ok(())
    }
    pub(super) fn check_link_target(&self, target: TextTarget) -> Result<(), CommandError> {
        let selection = self.resolve(target)?;
        let reason = if selection.range().is_empty() {
            "select_link"
        } else if !self.document().view.is_single_paragraph(selection.range()) {
            "link_one_paragraph"
        } else {
            return Ok(());
        };
        Err(CommandError::new(ErrorCode::Unavailable, reason))
    }
    pub(super) fn check_image_insertion(&self, target: TextTarget) -> Result<(), CommandError> {
        self.document()
            .view
            .validate_object_insertion(&self.resolve(target)?.range())?;
        Ok(())
    }
    pub(super) fn check_export_fields(&self) -> Result<(), CommandError> {
        if !self.session.metadata.ready_for_export() {
            return Err(CommandError::new(
                ErrorCode::Unavailable,
                "export_required_fields",
            ));
        }
        Ok(())
    }
}
