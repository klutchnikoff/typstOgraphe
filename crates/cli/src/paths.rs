//! Réancrage des seuls chemins typés : jamais le texte, les légendes ou les métadonnées.
use std::path::{Path, PathBuf};
use typstographe_core::engine::{Command, ImageInput, ModelInput};

pub fn relative_to(path: &mut PathBuf, base: &Path) {
    if path.is_relative() {
        *path = base.join(&*path);
    }
}
fn image(image: &mut ImageInput, base: &Path) {
    if let ImageInput::Path { path } = image {
        relative_to(path, base);
    }
}
pub fn command(command: &mut Command, base: &Path) {
    match command {
        Command::NewDocument(p) => {
            if let Some(value) = &mut p.signature {
                image(value, base);
            }
        }
        Command::InsertImage(p) => image(&mut p.image, base),
        Command::ReplaceImage(p) => image(&mut p.image, base),
        Command::SetSignature(p) => {
            if let Some(value) = &mut p.image {
                image(value, base);
            }
        }
        Command::SetModel(p) => {
            if let ModelInput::Path { path } = &mut p.model {
                relative_to(path, base);
            }
        }
        _ => {}
    }
}
