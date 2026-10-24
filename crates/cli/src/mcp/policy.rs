//! Le premier prototype ne peut lire ni écrire de fichiers documentaires.
use typstographe_core::engine::*;

pub fn exposed(spec: &CommandSpec) -> bool {
    !spec.id.as_str().starts_with("recovery.")
        && !matches!(
            spec.id.as_str(),
            "document.open"
                | "document.save"
                | "document.save_as"
                | "document.export"
                | "document.set_model"
                | "models.presentation"
        )
}

pub fn check(command: &Command, read_only: bool) -> Result<(), CommandError> {
    if read_only {
        return Err(CommandError::new(ErrorCode::Unavailable, "mcp_read_only"));
    }
    match command {
        Command::NewDocument(p) => embedded(p.signature.as_ref()),
        Command::InsertImage(p) => embedded(Some(&p.image)),
        Command::ReplaceImage(p) => embedded(Some(&p.image)),
        Command::SetSignature(p) => embedded(p.image.as_ref()),
        Command::Batch(p) if p.save.is_some() => denied(),
        command
            if !exposed(
                catalogue()
                    .iter()
                    .find(|s| s.id.as_str() == command.id())
                    .expect("registered command"),
            ) =>
        {
            denied()
        }
        _ => Ok(()),
    }
}
fn embedded(image: Option<&ImageInput>) -> Result<(), CommandError> {
    if matches!(image, Some(ImageInput::Path { .. })) {
        denied()
    } else {
        Ok(())
    }
}
pub fn denied<T>() -> Result<T, CommandError> {
    Err(CommandError::new(ErrorCode::Unavailable, "mcp_file_access"))
}
