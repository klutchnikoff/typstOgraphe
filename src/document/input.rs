//! Insertion et suppression dans la projection.
use super::*;
impl Projection {
    pub fn insert_text(&mut self, text: &str, char_index: usize) -> usize {
        if self.edit_error.is_some() {
            return 0;
        }
        let mut text = text.replace("\r\n", "\n").replace('\r', "\n");
        let at = char_index.min(self.len());
        let line = self.line_at(at);
        // Un séparateur de paragraphes occupe exactement une ligne vide.
        // Les retours excédentaires d’une saisie ou d’un collage sont superflus.
        if self.blocks[line] == BlockKind::Paragraph {
            let mut consecutive = self.text[..self.byte_index(at)]
                .chars()
                .rev()
                .take_while(|c| *c == '\n')
                .count()
                .min(
                    self.blocks[..line]
                        .iter()
                        .rev()
                        .take_while(|kind| **kind == BlockKind::Paragraph)
                        .count(),
                );
            text.retain(|c| {
                if c == '\n' {
                    if consecutive >= 2 {
                        return false;
                    }
                    consecutive += 1;
                } else {
                    consecutive = 0;
                }
                true
            });
            let following = self.text[self.byte_index(at)..]
                .chars()
                .take_while(|c| *c == '\n')
                .count()
                .min(
                    self.blocks[line + 1..]
                        .iter()
                        .take_while(|kind| **kind == BlockKind::Paragraph)
                        .count(),
                );
            let trailing = text.chars().rev().take_while(|c| *c == '\n').count();
            for _ in 0..trailing.min((consecutive + following).saturating_sub(2)) {
                text.pop();
            }
        }
        let count = text.chars().count();
        if count == 0 {
            return 0;
        }
        let image_before = match self.blocks[line] {
            BlockKind::Image(id) if at == self.line_ranges()[line].start => Some(id),
            _ => None,
        };
        if image_before.is_some() && !text.ends_with('\n') {
            text.push('\n');
        }
        let next_kind = match self.blocks[line] {
            BlockKind::Heading(_) | BlockKind::QuoteAttribution | BlockKind::Image(_) => {
                BlockKind::Paragraph
            }
            kind => kind,
        };
        let newlines = text.matches('\n').count();
        let old_id = self.line_ids[line];
        self.line_ids
            .splice(line + 1..line + 1, (0..newlines).map(|_| LineId::new()));
        // Insérer des lignes avant un bloc conserve l’identité du contenu déplacé.
        // Un partage au milieu conserve en revanche l’identité de la partie gauche.
        if newlines > 0
            && at == self.line_ranges()[line].start
            && !self.line_ranges()[line].is_empty()
        {
            self.line_ids[line] = LineId::new();
            self.line_ids[line + newlines] = old_id;
        }
        self.blocks
            .splice(line + 1..line + 1, std::iter::repeat_n(next_kind, newlines));
        self.text.insert_str(self.byte_index(at), &text);
        self.styles.splice(
            at..at,
            text.chars().map(|c| {
                if matches!(c, '\n' | '\t') {
                    InlineStyle::default()
                } else {
                    self.typing_style.clone()
                }
            }),
        );
        if let Some(id) = image_before {
            self.blocks[line] = BlockKind::Paragraph;
            self.blocks[line + newlines] = BlockKind::Image(id);
        }
        count
    }
    pub fn delete_char_range(&mut self, range: Range<usize>) {
        if !self.prepare_deletion(&range) {
            return;
        }
        let start = range.start.min(self.len());
        let end = range.end.min(self.len()).max(start);
        let line = self.line_at(start);
        let bytes = self.byte_index(start)..self.byte_index(end);
        let newlines = self.text[bytes.clone()].matches('\n').count();
        if start == self.line_ranges()[line].start && newlines > 0 {
            self.blocks[line] = self.blocks[line + newlines];
            self.line_ids[line] = self.line_ids[line + newlines];
        }
        self.blocks.drain(line + 1..line + newlines + 1);
        self.line_ids.drain(line + 1..line + newlines + 1);
        self.text.replace_range(bytes, "");
        self.styles.drain(start..end);
        self.clean_images();
    }
    #[allow(
        dead_code,
        reason = "Réutilisé par le diagnostic isolé et les tests de structure."
    )]
    pub fn replace_with(&mut self, text: &str) {
        let error = self.edit_error;
        *self = Self::default();
        self.edit_error = error;
        self.insert_text(text, 0);
    }
}
