use std::ops::Range;
/// Chaque caractère visible pointe vers sa plage d'octets dans le source réel.
/// Les commandes et les délimiteurs ne sont jamais comptés comme du texte.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceMap {
    pub characters: Vec<Range<usize>>,
    pub source_len: usize,
}
impl SourceMap {
    pub fn to_source(&self, character: usize) -> usize {
        self.characters
            .get(character)
            .map_or_else(|| self.characters.last().map_or(0, |r| r.end), |r| r.start)
    }
    pub fn to_visible(&self, byte: usize) -> usize {
        self.characters
            .iter()
            .enumerate()
            .min_by_key(|(_, r)| {
                if byte < r.start {
                    r.start - byte
                } else if byte >= r.end {
                    byte.saturating_sub(r.end) + 1
                } else {
                    0
                }
            })
            .map_or(0, |(i, r)| if byte >= r.end { i + 1 } else { i })
    }
}
