//! Keep the diff's semantic projection attached to its source leaves and atoms.
use super::{Highlight, Node, TextChange, TextChangeKind};
use std::ops::Range;

struct Segment {
    path: Vec<usize>,
    range: Range<usize>,
    text: bool,
}
pub(super) struct Projection {
    pub text: String,
    segments: Vec<Segment>,
}
impl Projection {
    pub fn new(nodes: &[Node]) -> Self {
        let mut result = Self {
            text: String::new(),
            segments: vec![],
        };
        for (i, node) in nodes.iter().enumerate() {
            if i > 0 {
                result.text.push('\n');
            }
            result.node(node, &mut vec![i]);
        }
        result
    }
    fn node(&mut self, node: &Node, path: &mut Vec<usize>) {
        let atom = match node.kind.as_str() {
            "footnote" => Some(format!(
                "[{}]",
                node.attrs
                    .as_ref()
                    .and_then(|a| a["text"].as_str())
                    .unwrap_or("")
            )),
            "citation" => Some(format!(
                "[{}]",
                node.attrs.as_ref().map_or(String::new(), |a| a.to_string())
            )),
            "softBreak" | "hardBreak" => Some("\n".into()),
            _ => None,
        };
        if let Some(value) = node.text.as_ref().or(atom.as_ref()) {
            let start = self.text.len();
            self.text.push_str(value);
            self.segments.push(Segment {
                path: path.clone(),
                range: start..self.text.len(),
                text: node.text.is_some(),
            });
            return;
        }
        let separated = !matches!(
            node.kind.as_str(),
            "paragraph" | "heading" | "figure" | "postscript" | "attachment"
        );
        for (i, child) in node.content.iter().enumerate() {
            if i > 0 && separated {
                self.text.push('\n');
            }
            path.push(i);
            self.node(child, path);
            path.pop();
        }
    }
    pub fn highlights(&self, changes: &[TextChange], kind: TextChangeKind) -> Vec<Highlight> {
        let mut result: Vec<Highlight> = vec![];
        let mut offset = 0;
        let mut first = 0;
        for change in changes {
            if change.kind != TextChangeKind::Equal && change.kind != kind {
                continue;
            }
            let end = offset + change.text.len();
            if change.kind == kind {
                while first < self.segments.len() && self.segments[first].range.end <= offset {
                    first += 1;
                }
                for segment in self.segments[first..]
                    .iter()
                    .take_while(|s| s.range.start < end)
                {
                    let from = offset.max(segment.range.start);
                    let to = end.min(segment.range.end);
                    if from >= to {
                        continue;
                    }
                    let (from, to) = if segment.text {
                        (
                            self.text[segment.range.start..from].encode_utf16().count() as u32,
                            self.text[segment.range.start..to].encode_utf16().count() as u32,
                        )
                    } else {
                        (0, 1)
                    };
                    if let Some(last) = result.last_mut()
                        && last.path == segment.path
                        && last.to >= from
                    {
                        last.to = last.to.max(to);
                    } else {
                        result.push(Highlight {
                            path: segment.path.clone(),
                            from,
                            to,
                        });
                    }
                }
            }
            offset = end;
        }
        result
    }
}
