//! Écriture commune des exports ; aucune mutation de la session documentaire.
use crate::errors::{CoreError, ExportError, StorageError, TemplateError};
use crate::{protocol::ExportFormat, rendering::RenderRequest};
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

/// Export an already validated structured document and its retained resources.
/// Existing files are never replaced; this is not a multi-file transaction.
pub fn write_document(
    document: &PagedDocument,
    request: &RenderRequest,
    path: &Path,
    format: ExportFormat,
    page_images: bool,
) -> Result<(), CoreError> {
    request.metadata.validate().map_err(CoreError::external)?;
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
    let assets = if format == ExportFormat::Pdf {
        Default::default()
    } else {
        request.assets.clone()
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
        let references = crate::bibliography::resources(&request.metadata);
        for (name, data) in request.dependencies.iter().chain(references.iter()) {
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
        for index in 0..document.pages().len() {
            write(
                path,
                &format!("page-{}.png", index + 1),
                &crate::rendering::page_png(document, index)?.expect("rendered page"),
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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::{
        assets::Assets,
        content::Node,
        document_types::{DocumentType, Metadata},
        rendering::TypstWorld,
    };
    use serde_json::json;

    #[test]
    fn formats_keep_their_requested_files_and_existing_destinations_are_untouched() {
        let root = tempfile::tempdir().unwrap();
        let assets = Assets::from([(
            "images/figure.svg".into(),
            include_bytes!("../../../fixtures/parc.svg")
                .as_slice()
                .into(),
        )]);
        let node: Node = serde_json::from_value(json!({"type":"doc","content":[
            {"type":"figure","attrs":{"src":"images/figure.svg","alt":"Parc arboré"},"content":[{"type":"text","text":"Le parc."}]}
        ]})).unwrap();
        let metadata = Metadata::demo(DocumentType::Courrier);
        let template = DocumentType::Courrier.default_model();
        let dependencies = crate::packages::for_template(&template);
        let source = crate::content::source(&node, &assets).unwrap();
        let mut world = TypstWorld::default();
        world.set_assets(&assets);
        world.set_dependencies(&dependencies);
        let document = world
            .compile_with_template(&source, &metadata, &template)
            .unwrap();
        let request = RenderRequest {
            source,
            metadata,
            template,
            dependencies,
            assets,
        };
        for (name, format, pdf, sources) in [
            ("pdf", ExportFormat::Pdf, true, false),
            ("typst", ExportFormat::Typst, false, true),
            ("both", ExportFormat::Both, true, true),
        ] {
            let path = root.path().join(name);
            write_document(&document, &request, &path, format, false).unwrap();
            assert_eq!(path.join("courrier.pdf").exists(), pdf);
            assert_eq!(path.join("main.typ").exists(), sources);
            assert_eq!(path.join("images/figure.svg").exists(), sources);
            if sources {
                assert_eq!(
                    std::fs::read(path.join("images/figure.svg")).unwrap(),
                    request.assets["images/figure.svg"].as_ref()
                );
                let mut reopened = TypstWorld::default();
                reopened.set_assets(&request.assets);
                reopened.set_dependencies(&request.dependencies);
                assert!(
                    reopened
                        .compile_with_template(
                            &std::fs::read_to_string(path.join("contenu.typ")).unwrap(),
                            &request.metadata,
                            &std::fs::read_to_string(path.join("modele.typ")).unwrap()
                        )
                        .is_ok()
                );
            }
            let sentinel = path.join("work-in-progress.txt");
            std::fs::write(&sentinel, "Ne pas remplacer.").unwrap();
            assert!(write_document(&document, &request, &path, format, false).is_err());
            assert_eq!(
                std::fs::read_to_string(&sentinel).unwrap(),
                "Ne pas remplacer."
            );
        }
        let file = root.path().join("existing-file");
        std::fs::write(&file, "Conserver.").unwrap();
        assert!(write_document(&document, &request, &file, ExportFormat::Both, false).is_err());
        assert_eq!(std::fs::read_to_string(file).unwrap(), "Conserver.");
        assert!(check_directory(Path::new("")).is_err());
        #[cfg(unix)]
        {
            let target = root.path().join("untouched");
            std::fs::create_dir(&target).unwrap();
            let link = root.path().join("link");
            std::os::unix::fs::symlink(&target, &link).unwrap();
            assert!(write_document(&document, &request, &link, ExportFormat::Both, false).is_err());
            assert_eq!(std::fs::read_dir(target).unwrap().count(), 0);
        }
    }
}
