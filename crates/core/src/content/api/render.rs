use super::{Session, content_error, decode, invalid};
use crate::{
    content::{
        Node,
        commands::{TextTarget, TextZone},
        contract::State,
    },
    engine::{CommandError, ErrorCode},
    navigation::{PagePoint, RenderSync},
};
use serde_json::{Value, json};
fn ranges(state: &State) -> Vec<(TextZone, usize, usize)> {
    fn visit(
        n: &Node,
        pos: usize,
        path: &mut Vec<usize>,
        out: &mut Vec<(TextZone, usize, usize)>,
    ) -> usize {
        if n.kind == "text" {
            return n.text.as_deref().unwrap_or("").encode_utf16().count();
        }
        if matches!(
            n.kind.as_str(),
            "footnote" | "citation" | "softBreak" | "hardBreak"
        ) {
            return 1;
        }
        let start = pos + usize::from(n.kind != "doc");
        let mut at = start;
        for (i, child) in n.content.iter().enumerate() {
            path.push(i);
            at += visit(child, at, path, out);
            path.pop();
        }
        if matches!(
            n.kind.as_str(),
            "paragraph"
                | "heading"
                | "figure"
                | "tableCaption"
                | "descriptionTerm"
                | "descriptionDefinition"
                | "quoteAttribution"
                | "postscript"
                | "attachment"
        ) {
            out.push((TextZone::Body { path: path.clone() }, start, at));
        }
        at - pos + usize::from(n.kind != "doc")
    }
    let mut out = vec![];
    visit(&state.document, 0, &mut vec![], &mut out);
    out
}
impl Session {
    pub(super) fn render_query(&mut self, id: &str, p: &Value) -> Result<Value, CommandError> {
        let archive = self.archive()?;
        let (source, map) = crate::content::source_mapped(&archive.document, &archive.assets)
            .map_err(|_| content_error(invalid("source_map")))?;
        self.world.set_assets(&archive.assets);
        self.world.set_dependencies(&archive.dependencies);
        let doc = self
            .world
            .compile_with_template(&source, &archive.metadata, &archive.template)
            .map_err(|e| CommandError::with_cause(ErrorCode::Template, "export_compile", e))?;
        let sync = RenderSync::new(self.guard().unwrap().revision, self.world.clone(), doc, map);
        let ranges = ranges(self.state()?);
        if let Some(target) = p.get("target") {
            let target: TextTarget = decode(target.clone())?;
            super::queries::text(self.state()?, Some(&target)).map_err(content_error)?;
            let (_, start, _) = ranges
                .iter()
                .find(|(zone, _, _)| *zone == target.zone)
                .ok_or_else(|| content_error(invalid("render_body_target")))?;
            if id == "render.selection_boxes" {
                return Ok(
                    json!({"guard":self.guard(),"boxes":sync.selection_highlights_exact(start+target.from as usize..start+target.to as usize)}),
                );
            }
            return Ok(
                json!({"guard":self.guard(),"point":sync.forward(start+target.from as usize)}),
            );
        }
        let point: PagePoint = decode(p["point"].clone())?;
        if point.point.iter().any(|n| !n.is_finite()) {
            return Err(content_error(invalid("point")));
        }
        let target = sync.backward(point).and_then(|pos| {
            ranges
                .iter()
                .find(|(_, from, to)| pos >= *from && pos <= *to)
                .map(|(zone, start, _)| TextTarget {
                    zone: zone.clone(),
                    from: (pos - start) as u32,
                    to: (pos - start) as u32,
                })
        });
        Ok(json!({"guard":self.guard(),"target":target}))
    }
}
