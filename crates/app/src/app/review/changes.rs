//! Presentation-only diff. Exact publication/position mapping stays in the engine.
use super::*;
use similar::{ChangeTag, TextDiff};
#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;

pub(super) struct ChangeRow {
    pub label: String,
    pub before: String,
    pub after: String,
}
#[derive(Default)]
pub(super) struct Changes {
    pub removed: Vec<Range<usize>>,
    pub added: Vec<Range<usize>>,
    pub rows: Vec<ChangeRow>,
    pub resources: Vec<String>,
}
fn push(ranges: &mut Vec<Range<usize>>, range: Range<usize>) {
    if let Some(last) = ranges.last_mut()
        && last.end == range.start
    {
        last.end = range.end;
    } else if !range.is_empty() {
        ranges.push(range);
    }
}
impl Changes {
    pub fn new(snapshot: &typstographe_mcp::ProposalReview) -> Self {
        let mut out = Self::default();
        let a = &snapshot.original.view;
        let b = &snapshot.candidate.view;
        let (mut old, mut new, mut old_line, mut new_line) = (0, 0, 0, 0);
        let diff = TextDiff::configure()
            .timeout(Duration::from_millis(80))
            .diff_unicode_words(&a.text, &b.text);
        let mut links = std::collections::BTreeSet::new();
        for change in diff.iter_all_changes() {
            let length = change.value().chars().count();
            match change.tag() {
                ChangeTag::Delete => {
                    push(&mut out.removed, old..old + length);
                    old += length;
                    old_line += change.value().matches('\n').count();
                }
                ChangeTag::Insert => {
                    push(&mut out.added, new..new + length);
                    new += length;
                    new_line += change.value().matches('\n').count();
                }
                ChangeTag::Equal => {
                    for ch in change.value().chars() {
                        if a.styles[old] != b.styles[new]
                            || a.blocks[old_line] != b.blocks[new_line]
                        {
                            push(&mut out.removed, old..old + 1);
                            push(&mut out.added, new..new + 1);
                        }
                        if a.styles[old].link != b.styles[new].link {
                            links.insert((
                                a.styles[old].link.clone().unwrap_or_default(),
                                b.styles[new].link.clone().unwrap_or_default(),
                            ));
                        }
                        old += 1;
                        new += 1;
                        if ch == '\n' {
                            old_line += 1;
                            new_line += 1;
                        }
                    }
                }
            }
        }
        for (before, after) in links {
            out.rows.push(ChangeRow {
                label: crate::tr!("cmd-link"),
                before,
                after,
            });
        }
        out.information(snapshot);
        out.resources(snapshot);
        if out.removed.is_empty()
            && out.added.is_empty()
            && snapshot.original.view.blocks != snapshot.candidate.view.blocks
        {
            let labels = |view: &Projection| {
                view.blocks
                    .iter()
                    .map(|kind| match kind {
                        BlockKind::Postscript => Command::InsertPostscript.label(),
                        BlockKind::Attachment => Command::InsertAttachment.label(),
                        BlockKind::Image(_) => Command::InsertImage.label(),
                        _ => Command::Block(kind.public_kind()).label(),
                    })
                    .collect::<Vec<_>>()
                    .join(" · ")
            };
            out.rows.push(ChangeRow {
                label: crate::tr!("review-structure"),
                before: labels(&snapshot.original.view),
                after: labels(&snapshot.candidate.view),
            });
        }
        out
    }
    fn row(&mut self, label: String, before: String, after: String) {
        if before != after {
            self.rows.push(ChangeRow {
                label,
                before,
                after,
            });
        }
    }
    fn information(&mut self, snapshot: &ProposalReview) {
        let out = self;
        let before = snapshot.before.metadata.field_values();
        for field in snapshot.after.metadata.field_values() {
            if let Some(old) = before.iter().find(|old| old.field == field.field)
                && old.value != field.value
            {
                out.rows.push(ChangeRow {
                    label: crate::i18n::text(field.label_key),
                    before: old.value.clone(),
                    after: field.value,
                });
            }
        }
        let (a, b) = (
            snapshot.before.metadata.options(),
            snapshot.after.metadata.options(),
        );
        out.row(
            crate::tr!("review-paper"),
            crate::i18n::text(a.paper.label_key()),
            crate::i18n::text(b.paper.label_key()),
        );
        out.row(
            crate::tr!("review-language"),
            crate::i18n::text(a.language.label_key()),
            crate::i18n::text(b.language.label_key()),
        );
        out.row(
            crate::tr!("review-orientation"),
            crate::i18n::text(if a.landscape {
                "orientation-landscape"
            } else {
                "orientation-portrait"
            }),
            crate::i18n::text(if b.landscape {
                "orientation-landscape"
            } else {
                "orientation-portrait"
            }),
        );
        out.row(
            crate::tr!("review-marker"),
            a.bullet_marker.clone().unwrap_or_default(),
            b.bullet_marker.clone().unwrap_or_default(),
        );
        if a.spelling_ignored != b.spelling_ignored {
            out.row(
                crate::tr!("review-dictionary"),
                a.spelling_ignored
                    .values()
                    .flat_map(|words| words.iter().cloned())
                    .collect::<Vec<_>>()
                    .join(", "),
                b.spelling_ignored
                    .values()
                    .flat_map(|words| words.iter().cloned())
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }
    }
    fn resources(&mut self, snapshot: &ProposalReview) {
        let out = self;
        if snapshot.before.template != snapshot.after.template
            || snapshot.before.dependencies != snapshot.after.dependencies
        {
            out.row(
                crate::tr!("review-model"),
                model_label(&snapshot.before.template),
                format!(
                    "{} · {}",
                    model_label(&snapshot.after.template),
                    crate::tr!("review-model-changed")
                ),
            );
        }
        if snapshot.before.metadata.signature_path() != snapshot.after.metadata.signature_path() {
            out.row(
                crate::tr!("review-signature"),
                snapshot.before.metadata.signature_path().map_or_else(
                    || crate::tr!("review-handwritten"),
                    |_| crate::tr!("review-signature-image"),
                ),
                snapshot.after.metadata.signature_path().map_or_else(
                    || crate::tr!("review-handwritten"),
                    |_| crate::tr!("review-signature-new"),
                ),
            );
        }
        for id in snapshot
            .original
            .view
            .images
            .keys()
            .chain(snapshot.candidate.view.images.keys())
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
        {
            let (a, b) = (
                snapshot.original.view.images.get(&id),
                snapshot.candidate.view.images.get(&id),
            );
            if a != b {
                out.row(
                    crate::tr!("image-alt-label"),
                    a.map(|i| i.alt.clone()).unwrap_or_default(),
                    b.map(|i| i.alt.clone()).unwrap_or_default(),
                );
            }
        }
        for path in snapshot
            .before
            .assets
            .keys()
            .chain(snapshot.after.assets.keys())
            .collect::<std::collections::BTreeSet<_>>()
        {
            if snapshot.before.assets.get(path) != snapshot.after.assets.get(path) {
                out.resources.push(path.clone());
            }
        }
    }
    pub fn information_count(&self) -> usize {
        self.rows.len() + self.resources.len()
    }
}
fn model_label(template: &str) -> String {
    for kind in DocumentType::ALL {
        for model in kind.models() {
            if model.source == template {
                return Command::Model(model.model).label();
            }
        }
    }
    crate::tr!("review-custom-model")
}
