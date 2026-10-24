//! Contextual reads replace Current with an explicit target, without touching primary state.
use super::*;
use serde_json::json;
fn bind(target: &mut TextTarget, selection: SelectionTarget) {
    if matches!(target, TextTarget::Current {}) {
        *target = selection.into();
    }
}
impl Engine {
    pub fn query_in_context(
        &self,
        context: &EditContext,
        mut query: Query,
    ) -> Result<Value, CommandError> {
        let state = self.editing_context(context)?;
        match &mut query {
            Query::Inspect(_) => {
                let mut value = self.query(query)?;
                value["selection"] = json!(state.selection);
                let style = state.typing_style;
                value["typing_style"] =
                    json!({"bold":style.bold,"italic":style.italic,"link":style.link});
                Ok(value)
            }
            Query::Commands(_) => Ok(self.commands_query_at(state.selection.into())),
            Query::Text(p) => {
                bind(&mut p.target, state.selection);
                self.query(query)
            }
            Query::Statistics(p) => {
                if let Some(target) = &mut p.target {
                    bind(target, state.selection);
                }
                self.query(query)
            }
            Query::Locate(p) => {
                if let RenderPosition::ToPreview { target } = &mut p.position {
                    bind(target, state.selection);
                }
                self.query(query)
            }
            Query::SelectionBoxes(p) => {
                bind(&mut p.target, state.selection);
                self.query(query)
            }
            _ => self.query(query),
        }
    }
}
impl Command {
    pub(in crate::engine) fn bind_current(&mut self, selection: TextTarget) {
        let target = match self {
            Self::InsertText(p) => &mut p.target,
            Self::InsertImage(p) => &mut p.target,
            Self::Bold(p) | Self::Italic(p) => &mut p.target,
            Self::SetLink(p) => &mut p.target,
            Self::SelectText(p) => &mut p.target,
            Self::FindNext(p) | Self::FindPrevious(p) => &mut p.target,
            Self::NextSpelling(p) | Self::PreviousSpelling(p) => &mut p.target,
            Self::DeleteText(p)
            | Self::SplitBlock(p)
            | Self::ExitBlock(p)
            | Self::Clear(p)
            | Self::RemoveLink(p)
            | Self::Paragraph(p)
            | Self::Heading1(p)
            | Self::Heading2(p)
            | Self::Heading3(p)
            | Self::Bullet(p)
            | Self::Numbered(p)
            | Self::Description(p)
            | Self::Quote(p) => &mut p.target,
            _ => return,
        };
        if matches!(target, TextTarget::Current {}) {
            *target = selection;
        }
    }
}
