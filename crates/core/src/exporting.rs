//! Écriture commune des exports ; aucune mutation de la session documentaire.
use crate::errors::{CoreError, ExportError, StorageError, TemplateError};
use crate::{
    engine::ExportFormat,
    rendering::{RenderRequest, retained_assets},
};
use std::path::Path;
use typst_layout::PagedDocument;

pub fn check_directory(path: &Path) -> Result<(), CoreError> {
    if path.as_os_str().is_empty() {
        return Err(CoreError::from(ExportError::Folder));
    }
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.is_dir() || metadata.file_type().is_symlink() => {
            Err(CoreError::from(ExportError::Nonempty))
        }
        Ok(_) => {
            if std::fs::read_dir(path)
                .map_err(CoreError::external)?
                .next()
                .is_some()
            {
                Err(CoreError::from(ExportError::Nonempty))
            } else {
                Ok(())
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(CoreError::external(e)),
    }
}

/// Le contrat ne promet pas une transaction multi-fichiers. On refuse toute cible
/// non vide et on utilise create_new pour ne jamais écraser un fichier existant.
pub fn write_document(
    document: &PagedDocument,
    request: &RenderRequest,
    path: &Path,
    format: ExportFormat,
    page_images: bool,
) -> Result<(), CoreError> {
    check_directory(path)?;
    // Préparer les opérations faillibles de compilation avant la première écriture.
    let pdf = if format != ExportFormat::Typst {
        Some(
            typst_pdf::pdf(document, &typst_pdf::PdfOptions::default())
                .map_err(|errors| {
                    errors
                        .iter()
                        .map(|e| e.message.to_string())
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .map_err(|error| CoreError::from(TemplateError::Compilation { error }))?,
        )
    } else {
        None
    };
    let assets = if format != ExportFormat::Pdf {
        retained_assets(
            &request.source,
            &request.metadata,
            &request.template,
            &request.assets,
            &request.dependencies,
        )?
    } else {
        Default::default()
    };
    std::fs::create_dir_all(path).map_err(CoreError::external)?;
    if let Some(pdf) = pdf {
        write(
            path,
            &format!("{}.pdf", request.metadata.document_type().file_stem()),
            &pdf,
        )?;
    }
    if format != ExportFormat::Pdf {
        write(path, "main.typ", request.metadata.main_source().as_bytes())?;
        write(path, "contenu.typ", request.source.as_bytes())?;
        if !assets.is_empty() {
            std::fs::create_dir(path.join("images")).map_err(CoreError::external)?;
        }
        for (name, data) in &assets {
            if !crate::assets::valid_path(name) {
                return Err(CoreError::from(StorageError::UnknownEntry));
            }
            write(path, name, data)?;
        }
        for (name, data) in request.dependencies.iter() {
            if let Some(parent) = path.join(name).parent() {
                std::fs::create_dir_all(parent).map_err(CoreError::external)?;
            }
            write(path, name, data)?;
        }
        write(path, "modele.typ", request.template.as_bytes())?;
        write(
            path,
            "metadonnees.json",
            serde_json::to_string_pretty(&request.metadata)
                .map_err(CoreError::external)?
                .as_bytes(),
        )?;
        crate::fonts::export_used(document, path).map_err(CoreError::external)?;
    }
    if page_images {
        for (index, page) in document.pages().iter().enumerate() {
            let pixmap = typst_render::render(page, &typst_render::RenderOptions::default());
            write(
                path,
                &format!("page-{}.png", index + 1),
                &pixmap.encode_png().map_err(CoreError::external)?,
            )?;
        }
    }
    Ok(())
}
fn write(path: &Path, name: &str, data: &[u8]) -> Result<(), CoreError> {
    use std::io::Write;
    let mut file = std::fs::File::create_new(path.join(name)).map_err(|e| {
        CoreError::from(ExportError::WriteFailed {
            name: (name).to_string(),
            error: (e.to_string()).to_string(),
        })
    })?;
    file.write_all(data).map_err(|e| {
        CoreError::from(ExportError::WriteFailed {
            name: (name).to_string(),
            error: (e.to_string()).to_string(),
        })
    })
}
