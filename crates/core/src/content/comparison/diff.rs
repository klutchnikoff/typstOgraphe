use super::*;
use similar::{Algorithm, DiffOp, capture_diff_slices_deadline};
use unicode_segmentation::UnicodeSegmentation;
/// Bounded matching; a deadline falls back to conservative insertions/deletions.
pub(in crate::content) fn sequence_diff<T: Eq + std::hash::Hash + Ord>(
    old: &[T],
    new: &[T],
) -> Vec<DiffOp> {
    capture_diff_slices_deadline(
        Algorithm::Myers,
        old,
        new,
        Some(std::time::Instant::now() + std::time::Duration::from_millis(100)),
    )
}
fn text(node: &Node) -> String {
    if let Some(s) = &node.text {
        return s.clone();
    }
    match node.kind.as_str() {
        "footnote" => format!(
            "[{}]",
            node.attrs
                .as_ref()
                .and_then(|a| a["text"].as_str())
                .unwrap_or("")
        ),
        "citation" => format!(
            "[{}]",
            node.attrs.as_ref().map_or(String::new(), |a| a.to_string())
        ),
        "softBreak" | "hardBreak" => "\n".into(),
        _ => node.content.iter().map(text).collect::<Vec<_>>().join(
            if matches!(
                node.kind.as_str(),
                "paragraph" | "heading" | "figure" | "postscript" | "attachment"
            ) {
                ""
            } else {
                "\n"
            },
        ),
    }
}
pub(super) fn words(before: &[Node], after: &[Node]) -> Vec<TextChange> {
    let a = before.iter().map(text).collect::<Vec<_>>().join("\n");
    let b = after.iter().map(text).collect::<Vec<_>>().join("\n");
    let old: Vec<_> = a.split_word_bounds().collect();
    let new: Vec<_> = b.split_word_bounds().collect();
    sequence_diff(&old, &new)
        .into_iter()
        .flat_map(|op| {
            let (tag, a, b) = op.as_tag_tuple();
            let mut out = vec![];
            if tag == similar::DiffTag::Equal {
                out.push(TextChange {
                    kind: TextChangeKind::Equal,
                    text: old[a].concat(),
                });
            } else {
                if !a.is_empty() {
                    out.push(TextChange {
                        kind: TextChangeKind::Delete,
                        text: old[a].concat(),
                    });
                }
                if !b.is_empty() {
                    out.push(TextChange {
                        kind: TextChangeKind::Insert,
                        text: new[b].concat(),
                    });
                }
            }
            out
        })
        .collect()
}
fn linked(node: &Node) -> bool {
    !matches!(
        node.kind.as_str(),
        "paragraph" | "heading" | "text" | "softBreak" | "hardBreak"
    ) || node.content.iter().any(linked)
}
pub(super) fn groups(before: &Archive, after: &Archive) -> Vec<Group> {
    let a: Vec<_> = before
        .document
        .content
        .iter()
        .map(|n| serde_json::to_string(n).unwrap())
        .collect();
    let b: Vec<_> = after
        .document
        .content
        .iter()
        .map(|n| serde_json::to_string(n).unwrap())
        .collect();
    let coupled =
        before.state().settings != after.state().settings || before.assets != after.assets;
    if coupled {
        return vec![Group {
            id: 0,
            before: 0..a.len(),
            after: 0..b.len(),
            section: Section::Document,
            linked: true,
            changes: words(&before.document.content, &after.document.content),
            decision: Decision::Pending,
        }];
    }
    sequence_diff(&a, &b)
        .into_iter()
        .filter(|op| op.tag() != similar::DiffTag::Equal)
        .enumerate()
        .map(|(id, op)| {
            let ar = op.old_range();
            let br = op.new_range();
            Group {
                id,
                changes: words(
                    &before.document.content[ar.clone()],
                    &after.document.content[br.clone()],
                ),
                linked: before.document.content[ar.clone()]
                    .iter()
                    .chain(&after.document.content[br.clone()])
                    .any(linked),
                before: ar,
                after: br,
                section: Section::Body,
                decision: Decision::Pending,
            }
        })
        .collect()
}

pub(super) fn files(before: &Archive, after: &Archive) -> Vec<FileChange> {
    use sha2::{Digest, Sha256};
    fn state(bytes: &[u8]) -> FileState {
        FileState {
            bytes: bytes.len(),
            hash: Sha256::digest(bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
        }
    }
    let mut result = vec![];
    for (kind, left, right) in [
        ("image", before.assets.clone(), after.assets.clone()),
        (
            "bibliography",
            crate::bibliography::resources(&before.metadata),
            crate::bibliography::resources(&after.metadata),
        ),
    ] {
        let paths: std::collections::BTreeSet<_> = left.keys().chain(right.keys()).collect();
        for path in paths {
            if left.get(path) != right.get(path) {
                result.push(FileChange {
                    kind: kind.into(),
                    path: path.clone(),
                    before: left.get(path).map(|v| state(v)),
                    after: right.get(path).map(|v| state(v)),
                });
            }
        }
    }
    if before.template != after.template || before.model_id != after.model_id {
        result.push(FileChange {
            kind: "model".into(),
            path: "model.typ".into(),
            before: Some(state(before.template.as_bytes())),
            after: Some(state(after.template.as_bytes())),
        });
    }
    result
}
