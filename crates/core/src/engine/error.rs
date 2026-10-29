use crate::document::{edits::EditError, structure::TargetError};
pub use crate::protocol::{CommandError, ErrorCode};
impl From<TargetError> for CommandError {
    fn from(error: TargetError) -> Self {
        Self::new(
            ErrorCode::Document,
            match error {
                TargetError::ForeignDocument => "foreign_document",
                TargetError::StaleRevision => "stale_revision",
                TargetError::InvalidPosition => "invalid_selection",
                TargetError::UnknownBlock => "unknown_block",
                TargetError::UnknownZone => "unknown_zone",
            },
        )
    }
}
impl From<EditError> for CommandError {
    fn from(error: EditError) -> Self {
        if let EditError::Target(error) = error {
            return error.into();
        }
        Self::new(
            ErrorCode::Document,
            match error {
                EditError::ProtectedBoundary => "protected_boundary",
                EditError::ReadOnly => "file_read_only",
                EditError::InvalidText => "invalid_text",
                EditError::StaleDraft => "stale_revision",
                EditError::InvalidSelection => "invalid_selection",
                _ => "invalid_projection",
            },
        )
    }
}
