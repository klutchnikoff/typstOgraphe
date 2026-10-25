//! UI preflight and response envelopes; expensive rendering stays on a worker.
use super::*;
impl Host<'_> {
    pub(crate) fn refresh_read_reply(&self, mut response: Reply) -> Reply {
        response.value["document"] = json!(self.reference());
        response.value["permissions"] = self.policy.describe();
        response
    }
    pub(crate) fn prepare_render_read(
        &self,
        name: &str,
        arguments: &mut Map<String, Value>,
    ) -> Result<Option<RenderTask>, CommandError> {
        if serde_json::to_vec(arguments)
            .map_err(CommandError::invalid)?
            .len()
            > MAX_BATCH_BYTES
        {
            return Err(CommandError::new(
                ErrorCode::InvalidParams,
                "batch_size_limit",
            ));
        }
        let document = arguments
            .get("document")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(CommandError::invalid)?;
        self.check_document(document, true)?;
        if name == "render.page" {
            let _: RenderPage = decode(arguments.clone())?;
        }
        if name == "render.compile" {
            // This query has no render parameter, so the common preparation and
            // typed decode validate the full call without triggering compilation.
            let mut params = arguments.clone();
            self.prepare_registered(name, &mut params)?;
            Query::from_call(name, Value::Object(params))?;
        }
        let needs_current =
            matches!(name, "render.page" | "render.compile") || !arguments.contains_key("render");
        if needs_current && self.engine.render_info().is_none() {
            self.capture_render_target(name, arguments)?;
            return Ok(Some(self.engine.prepare_render(GuardParams {
                guard: self.engine.target_guard(),
            })?));
        }
        Ok(None)
    }
    fn capture_render_target(
        &self,
        name: &str,
        arguments: &mut Map<String, Value>,
    ) -> Result<(), CommandError> {
        if arguments.contains_key("selector") {
            return Ok(());
        }
        let Some(context) = &self.context else {
            return Ok(());
        };
        let selection = self.engine.editing_context(context)?.selection;
        let target = if name == "render.selection_boxes" {
            Some(
                arguments
                    .entry("target")
                    .or_insert_with(|| json!({"kind":"current"})),
            )
        } else if name == "render.locate"
            && arguments
                .get("position")
                .and_then(|p| p.get("direction"))
                .is_some_and(|d| d == "to_preview")
        {
            arguments
                .get_mut("position")
                .and_then(Value::as_object_mut)
                .map(|p| {
                    p.entry("target")
                        .or_insert_with(|| json!({"kind":"current"}))
                })
        } else {
            None
        };
        if let Some(target) = target {
            let parsed: TextTarget =
                serde_json::from_value(target.clone()).map_err(CommandError::invalid)?;
            if matches!(parsed, TextTarget::Current {}) {
                *target = json!(TextTarget::from(selection));
            }
        }
        Ok(())
    }
    pub(crate) fn compiled_reply(&self, result: CompileResult) -> Reply {
        self.reply(
            "render.compile",
            self.query_response("render.compile", json!(result)),
            None,
        )
    }
    pub(crate) fn begin_page(
        &self,
        arguments: Map<String, Value>,
    ) -> Result<(PageImageTask, Reply), CommandError> {
        let p: RenderPage = decode(arguments)?;
        self.check_document(Some(p.document), true)?;
        let render = self
            .engine
            .render_info()
            .ok_or_else(|| CommandError::new(ErrorCode::Template, "render_unavailable"))?;
        let index = p
            .page
            .checked_sub(1)
            .map(|n| n as usize)
            .filter(|i| *i < render.pages.len())
            .ok_or_else(|| CommandError::new(ErrorCode::InvalidParams, "cli_render_page"))?;
        let task = self.engine.prepare_page_image(render.reference, index)?;
        let response = self.reply(
            "render.page",
            json!({"page":p.page,"pages":render.pages.len(),"size_points":render.pages[index]}),
            None,
        );
        Ok((task, response))
    }
}
