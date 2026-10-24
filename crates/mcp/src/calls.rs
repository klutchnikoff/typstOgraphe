use typstographe_core::engine::*;

pub fn compile_render(engine: &Engine) -> Result<RenderInfo, CommandError> {
    let compiled = engine.compile(GuardParams {
        guard: engine.target_guard(),
    })?;
    compiled.render.ok_or_else(|| {
        compiled
            .diagnostics
            .into_iter()
            .find_map(|d| d.cause)
            .map(|cause| CommandError::with_cause(ErrorCode::Template, "render_compile", cause))
            .unwrap_or_else(|| CommandError::new(ErrorCode::Template, "render_unavailable"))
    })
}
