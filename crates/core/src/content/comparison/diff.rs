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
fn words(before: &[Node], after: &[Node]) -> (Vec<TextChange>, Vec<Highlight>, Vec<Highlight>) {
    let a = super::projection::Projection::new(before);
    let b = super::projection::Projection::new(after);
    let old: Vec<_> = a.text.split_word_bounds().collect();
    let new: Vec<_> = b.text.split_word_bounds().collect();
    let changes = sequence_diff(&old, &new)
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
        .collect::<Vec<_>>();
    let before = a.highlights(&changes, TextChangeKind::Delete);
    let after = b.highlights(&changes, TextChangeKind::Insert);
    (changes, before, after)
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
        let (changes, before_highlights, after_highlights) =
            words(&before.document.content, &after.document.content);
        return vec![Group {
            id: 0,
            before: 0..a.len(),
            after: 0..b.len(),
            section: Section::Document,
            linked: true,
            changes,
            before_highlights,
            after_highlights,
            decision: Decision::Pending,
        }];
    }
    let boundaries = super::grouping::Boundaries::new(before, after);
    sequence_diff(&a, &b)
        .into_iter()
        .filter(|op| op.tag() != similar::DiffTag::Equal)
        .flat_map(|op| boundaries.split(op.old_range(), op.new_range()))
        .enumerate()
        .map(|(id, (ar, br))| {
            let (changes, before_highlights, after_highlights) = words(
                &before.document.content[ar.clone()],
                &after.document.content[br.clone()],
            );
            Group {
                id,
                changes,
                before_highlights,
                after_highlights,
                linked: ar.len() > 1
                    || br.len() > 1
                    || before.document.content[ar.clone()]
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
