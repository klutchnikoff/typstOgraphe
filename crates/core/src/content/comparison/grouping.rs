//! Split adjacent edits only when the same block boundaries survive in both states.
use super::{Archive, Range};
use crate::content::shared::SharedDocument;
use yrs::{Transact, XmlFragment, branch::BranchID};

fn identities(archive: &Archive) -> Option<Vec<BranchID>> {
    let shared = SharedDocument::from_update(&archive.yjs).ok()?;
    let root = shared.doc.get_or_insert_xml_fragment("default");
    let tx = shared.doc.transact();
    (0..archive.document.content.len())
        .map(|i| root.get(&tx, i as u32).map(|node| node.id()))
        .collect()
}

pub(super) struct Boundaries {
    before: Option<Vec<BranchID>>,
    after: Option<Vec<BranchID>>,
}
impl Boundaries {
    pub fn new(before: &Archive, after: &Archive) -> Self {
        Self {
            before: identities(before),
            after: identities(after),
        }
    }
    pub fn split(
        &self,
        before: Range<usize>,
        after: Range<usize>,
    ) -> Vec<(Range<usize>, Range<usize>)> {
        // A split/join, insertion/deletion or a recreated boundary is not a set
        // of independent paragraph replacements. Never infer identity from text.
        let same_boundaries = before.len() > 1
            && before.len() == after.len()
            && self
                .before
                .as_ref()
                .zip(self.after.as_ref())
                .is_some_and(|(a, b)| a[before.clone()] == b[after.clone()]);
        if same_boundaries {
            before
                .zip(after)
                .map(|(a, b)| (a..a + 1, b..b + 1))
                .collect()
        } else {
            vec![(before, after)]
        }
    }
}
