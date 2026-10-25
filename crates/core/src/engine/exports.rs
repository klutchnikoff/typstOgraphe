//! Même export pour un appel synchrone et une tâche de fond de l’interface.
use super::*;
use crate::rendering::{RenderRequest, TypstWorld};
use serde_json::json;

pub struct ExportTask {
    request: RenderRequest,
    params: ExportParams,
}
/// Compiled immutable snapshot. No output files exist until `write` is called.
pub struct CompiledExport {
    document: typst_layout::PagedDocument,
    request: RenderRequest,
    params: ExportParams,
}
impl ExportTask {
    pub fn execute(self) -> Result<Value, CommandError> {
        self.compile()?.write()
    }
    pub fn compile(self) -> Result<CompiledExport, CommandError> {
        let mut world = TypstWorld::default();
        world.set_assets(&self.request.assets);
        world.set_dependencies(&self.request.dependencies);
        let document = world
            .compile_with_template(
                &self.request.source,
                &self.request.metadata,
                &self.request.template,
            )
            .map_err(|e| CommandError::with_cause(ErrorCode::Template, "export_compile", e))?;
        Ok(CompiledExport {
            document,
            request: self.request,
            params: self.params,
        })
    }
}
impl CompiledExport {
    pub fn write(self) -> Result<Value, CommandError> {
        crate::exporting::write_document(
            &self.document,
            &self.request,
            &self.params.directory,
            self.params.format,
            self.params.page_images,
        )
        .map_err(|e| CommandError::with_cause(ErrorCode::Io, "file_export", e))?;
        comemo::evict(10);
        Ok(
            json!({"directory":self.params.directory,"format":self.params.format,"page_images":self.params.page_images,"guard":self.params.guard}),
        )
    }
}
impl Engine {
    /// Une copie immuable du document validé ; une édition ultérieure ne change pas l’export demandé.
    pub fn prepare_export(&self, params: ExportParams) -> Result<ExportTask, CommandError> {
        self.check_guard(params.guard)?;
        if !self.open {
            return Err(CommandError::new(ErrorCode::Unavailable, "no_document"));
        }
        self.check_export_fields()?;
        crate::packages::validate(&self.session.template, &self.session.dependencies)
            .map_err(|e| CommandError::with_cause(ErrorCode::Template, "model_dependencies", e))?;
        crate::exporting::check_directory(&params.directory)
            .map_err(|e| CommandError::with_cause(ErrorCode::Io, "file_export", e))?;
        Ok(ExportTask {
            request: RenderRequest {
                revision: self.session.revision,
                source: self.session.document.source().into(),
                source_map: self.session.document.source_map.clone(),
                metadata: self.session.metadata.clone(),
                template: self.session.template.clone(),
                assets: self.session.document.assets.clone(),
                dependencies: self.session.dependencies.clone(),
                export_dir: None,
            },
            params,
        })
    }
}
