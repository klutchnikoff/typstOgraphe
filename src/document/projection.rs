//! Projection visible, repérage et opérations de blocs et styles.
use super::*;
impl Projection {
    pub(super) fn same_content(&self, other: &Self) -> bool {
        self.text == other.text
            && self.styles == other.styles
            && self.blocks == other.blocks
            && self.images == other.images
    }
    pub fn len(&self) -> usize {
        self.styles.len()
    }
    pub fn is_empty(&self) -> bool {
        self.styles.is_empty()
    }
    pub fn byte_index(&self, index: usize) -> usize {
        self.text
            .char_indices()
            .nth(index)
            .map_or(self.text.len(), |(i, _)| i)
    }
    pub fn line_at(&self, index: usize) -> usize {
        self.text
            .chars()
            .take(index.min(self.len()))
            .filter(|c| *c == '\n')
            .count()
    }
    pub fn line_ranges(&self) -> Vec<Range<usize>> {
        let mut start = 0;
        self.text
            .split('\n')
            .map(|s| {
                let end = start + s.chars().count();
                let range = start..end;
                start = end + 1;
                range
            })
            .collect()
    }
    pub fn selected_lines(&self, range: Range<usize>) -> Range<usize> {
        let first = self.line_at(range.start);
        let last = self.line_at(if range.is_empty() {
            range.end
        } else {
            range.end.saturating_sub(1)
        });
        first..last + 1
    }
    /// Un paragraphe peut couvrir plusieurs lignes de texte courant ou cité.
    /// Les lignes vides et les structures (titres, éléments, annotations) le bornent.
    pub fn paragraph_at(&self, position: usize) -> Range<usize> {
        let ranges = self.line_ranges();
        let line = self.line_at(position);
        let kind = self.blocks[line];
        let mut first = line;
        let mut last = line;
        if matches!(
            kind,
            BlockKind::Paragraph | BlockKind::Quote | BlockKind::Postscript
        ) && !ranges[line].is_empty()
        {
            while first > 0 && self.blocks[first - 1] == kind && !ranges[first - 1].is_empty() {
                first -= 1;
            }
            while last + 1 < ranges.len()
                && self.blocks[last + 1] == kind
                && !ranges[last + 1].is_empty()
            {
                last += 1;
            }
        }
        ranges[first].start..ranges[last].end
    }
    pub fn is_single_paragraph(&self, range: Range<usize>) -> bool {
        let paragraph = self.paragraph_at(range.start);
        range.end <= paragraph.end
    }
    pub fn style_at(&self, position: usize) -> InlineStyle {
        self.styles
            .get(position)
            .or_else(|| self.styles.last())
            .cloned()
            .unwrap_or_default()
    }
    pub fn set_block(&mut self, range: Range<usize>, kind: BlockKind) {
        let first = self.paragraph_at(range.start).start;
        let last = self
            .paragraph_at(if range.is_empty() {
                range.end
            } else {
                range.end.saturating_sub(1)
            })
            .end;
        let target = first..last;
        // Un titre ou un élément représente un paragraphe, pas chacune de ses
        // lignes de saisie. Joindre les retours simples conserve tous les indices.
        if matches!(kind, BlockKind::Heading(_)) || kind.is_list() {
            let ranges = self.line_ranges();
            let lines = self.selected_lines(target.clone());
            for line in (lines.start..lines.end.saturating_sub(1)).rev() {
                if matches!(self.blocks[line], BlockKind::Paragraph | BlockKind::Quote)
                    && self.blocks[line] == self.blocks[line + 1]
                    && !ranges[line].is_empty()
                    && !ranges[line + 1].is_empty()
                {
                    let byte = self.byte_index(ranges[line].end);
                    self.text.replace_range(byte..byte + 1, " ");
                    self.blocks.remove(line + 1);
                    self.line_ids.remove(line + 1);
                }
            }
        }
        let lines = self.line_ranges();
        let selected = !range.is_empty();
        for index in self.selected_lines(target) {
            if selected
                && lines[index].is_empty()
                && self.blocks[index] == BlockKind::Paragraph
                && kind != BlockKind::Quote
            {
                continue;
            }
            if matches!(self.blocks[index], BlockKind::Image(_)) {
                continue;
            }
            if kind == BlockKind::Quote && self.blocks[index] == BlockKind::QuoteAttribution {
                continue;
            }
            if self.blocks[index] == BlockKind::Description && kind != BlockKind::Description {
                let line = self.line_ranges()[index].clone();
                if let Some(at) = self
                    .text
                    .chars()
                    .skip(line.start)
                    .take(line.len())
                    .position(|c| c == '\t')
                {
                    let byte = self.byte_index(line.start + at);
                    self.text.replace_range(byte..byte + 1, " ");
                }
            }
            self.blocks[index] = kind;
        }
    }
    /// Un caractère objet représente l’image ; le reste du paragraphe est sa légende.
    pub fn insert_image(&mut self, at: usize, image: ImageBlock) -> usize {
        if let Err(error) = self.validate_object_insertion(&(at..at)) {
            self.edit_error = Some(error);
            return at;
        }
        let id = self.images.keys().next_back().copied().unwrap_or(0) + 1;
        let at = at.min(self.len());
        let line = self.line_at(at);
        let range = self.line_ranges()[line].clone();
        let old_kind = self.blocks[line];
        let before_block = at == range.start && old_kind != BlockKind::QuoteAttribution;
        let prefix = if before_block { "" } else { "\n" };
        let text = format!("{prefix}{IMAGE_OBJECT}\n");
        self.insert_text(&text, at);
        if before_block && range.is_empty() {
            self.line_ids.swap(line, line + 1);
        }
        let position = at + prefix.len();
        let line = self.line_at(position);
        self.blocks[line] = BlockKind::Image(id);
        if before_block {
            self.blocks[line + 1] = old_kind;
        } else if !matches!(self.blocks[line + 1], BlockKind::Image(_)) {
            self.blocks[line + 1] = BlockKind::Paragraph;
        }
        self.images.insert(id, image);
        position
    }
    pub fn image_range(&self, id: u64) -> Option<Range<usize>> {
        let line = self
            .blocks
            .iter()
            .position(|kind| *kind == BlockKind::Image(id))?;
        Some(self.line_ranges()[line].clone())
    }
    // Une jonction de paragraphes ou la suppression du caractère objet libère
    // le texte de légende. Aucun marqueur interne n’est exporté comme du texte.
    pub(super) fn clean_images(&mut self) {
        let ranges = self.line_ranges();
        let characters: Vec<_> = self.text.chars().collect();
        for (line, kind) in self.blocks.iter_mut().enumerate() {
            if matches!(kind, BlockKind::Image(_))
                && characters.get(ranges[line].start) != Some(&IMAGE_OBJECT)
            {
                *kind = BlockKind::Paragraph;
            }
        }
        let active: std::collections::BTreeSet<_> = self
            .blocks
            .iter()
            .filter_map(|k| {
                if let BlockKind::Image(id) = k {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();
        self.images.retain(|id, _| active.contains(id));
        // Une suppression qui rejoint le paragraphe précédent retire aussi
        // l’objet orphelin, tout en conservant le texte de sa légende.
        for at in (0..characters.len()).rev() {
            let line = self.line_at(at);
            if characters[at] == IMAGE_OBJECT
                && !(at == ranges[line].start && matches!(self.blocks[line], BlockKind::Image(_)))
            {
                let byte = self.byte_index(at);
                self.text
                    .replace_range(byte..byte + IMAGE_OBJECT.len_utf8(), "");
                self.styles.remove(at);
            }
        }
    }
    /// Blocs textuels de fin de courrier ; les lignes de P.J. sont des mentions.
    pub fn letter_note_groups(&self) -> Vec<(BlockKind, Range<usize>)> {
        let mut groups = Vec::new();
        let mut line = 0;
        while line < self.blocks.len() {
            let kind = self.blocks[line];
            let first = line;
            line += 1;
            if kind.is_letter_note() {
                while self.blocks.get(line) == Some(&kind) {
                    line += 1;
                }
                groups.push((kind, first..line));
            }
        }
        groups
    }
    pub fn insert_letter_note(&mut self, kind: BlockKind) -> usize {
        assert!(kind.is_letter_note());
        let ranges = self.line_ranges();
        if let Some((_, lines)) = self
            .letter_note_groups()
            .into_iter()
            .find(|(k, _)| *k == kind)
        {
            if kind == BlockKind::Postscript {
                return ranges[lines.start].start;
            }
            let last = &ranges[lines.end - 1];
            if last.is_empty() {
                return last.start;
            }
            let at = last.end;
            self.insert_text("\n", at);
            return at + 1;
        }
        if kind == BlockKind::Postscript
            && let Some(line) = self
                .blocks
                .iter()
                .position(|kind| *kind == BlockKind::Attachment)
        {
            let at = ranges[line].start;
            self.insert_text("\n\n", at);
            self.blocks[line] = BlockKind::Postscript;
            self.blocks[line + 1] = BlockKind::Paragraph;
            return at;
        }
        let at = self.len();
        let separator = if self.text.is_empty() || self.text.ends_with("\n\n") {
            ""
        } else if self.text.ends_with('\n') {
            "\n"
        } else {
            "\n\n"
        };
        let count = self.insert_text(separator, at);
        *self.blocks.last_mut().unwrap() = kind;
        at + count
    }
    pub fn quote_groups(&self) -> Vec<QuoteGroup> {
        let mut groups = Vec::new();
        let mut line = 0;
        while line < self.blocks.len() {
            if self.blocks[line] != BlockKind::Quote {
                line += 1;
                continue;
            }
            let start = line;
            while self.blocks.get(line) == Some(&BlockKind::Quote) {
                line += 1;
            }
            let attribution =
                (self.blocks.get(line) == Some(&BlockKind::QuoteAttribution)).then_some(line);
            groups.push(QuoteGroup {
                body: start..line,
                attribution,
            });
            line += usize::from(attribution.is_some());
        }
        groups
    }
    pub(super) fn ensure_quote_attributions(&mut self) -> Vec<usize> {
        for index in 0..self.blocks.len() {
            if self.blocks[index] == BlockKind::QuoteAttribution
                && (index == 0 || self.blocks[index - 1] != BlockKind::Quote)
            {
                self.blocks[index] = BlockKind::Paragraph;
            }
        }
        let mut inserted = Vec::new();
        for group in self
            .quote_groups()
            .into_iter()
            .rev()
            .filter(|g| g.attribution.is_none())
        {
            let end = self.line_ranges()[group.body.end - 1].end;
            self.insert_text("\n", end);
            self.blocks[group.body.end] = BlockKind::QuoteAttribution;
            inserted.push(end);
        }
        inserted
    }
    /// Le séparateur est une frontière éditable, jamais un texte indicatif.
    /// Les insertions sont renvoyées dans l’ordre pour réaligner les curseurs.
    pub(super) fn ensure_description_separators(&mut self) -> Vec<usize> {
        let mut inserted = Vec::new();
        for (index, range) in self.line_ranges().into_iter().enumerate().rev() {
            if self.blocks[index] == BlockKind::Description
                && !self.text[self.byte_index(range.start)..self.byte_index(range.end)]
                    .contains('\t')
            {
                self.insert_text("\t", range.end);
                inserted.push(range.end);
            }
        }
        inserted
    }
    /// Sans ligne vide, Typst lirait le texte suivant comme la suite de la liste.
    pub(super) fn ensure_list_boundaries(&mut self) -> Vec<usize> {
        let mut inserted = Vec::new();
        for (line, range) in self.line_ranges().into_iter().enumerate().rev() {
            if line > 0
                && self.blocks[line - 1].is_list()
                && self.blocks[line] == BlockKind::Paragraph
                && !range.is_empty()
            {
                let at = range.start - 1;
                self.insert_text("\n", at);
                self.blocks[line] = BlockKind::Paragraph;
                inserted.push(at);
            }
        }
        inserted
    }
    pub fn mark_state(&self, range: Range<usize>, mark: Mark) -> (bool, bool) {
        let enabled = |s: &InlineStyle| match mark {
            Mark::Bold => s.bold,
            Mark::Italic => s.italic,
        };
        if range.is_empty() {
            return (enabled(&self.typing_style), false);
        }
        let items: Vec<_> = self
            .styles
            .iter()
            .zip(self.text.chars())
            .skip(range.start.min(self.len()))
            .take(range.end.min(self.len()) - range.start.min(self.len()))
            .filter(|(_, c)| !matches!(*c, '\n' | '\t' | IMAGE_OBJECT))
            .map(|(style, _)| style)
            .collect();
        let any = items.iter().any(|style| enabled(style));
        (
            any && items.iter().all(|style| enabled(style)),
            any && !items.iter().all(|style| enabled(style)),
        )
    }
    pub fn toggle_mark(&mut self, range: Range<usize>, mark: Mark) {
        let value = !self.mark_state(range.clone(), mark).0;
        self.set_mark(range, mark, value);
    }
    pub fn set_mark(&mut self, range: Range<usize>, mark: Mark, value: bool) {
        let apply = |s: &mut InlineStyle| match mark {
            Mark::Bold => s.bold = value,
            Mark::Italic => s.italic = value,
        };
        if range.is_empty() {
            apply(&mut self.typing_style);
        } else {
            for (s, c) in self.styles[range.clone()]
                .iter_mut()
                .zip(self.text.chars().skip(range.start))
            {
                if !matches!(c, '\n' | '\t' | IMAGE_OBJECT) {
                    apply(s);
                }
            }
        }
    }
    pub fn set_link(&mut self, range: Range<usize>, link: Option<String>) {
        for (s, c) in self.styles[range.clone()]
            .iter_mut()
            .zip(self.text.chars().skip(range.start))
        {
            if !matches!(c, '\n' | '\t' | IMAGE_OBJECT) {
                s.link.clone_from(&link);
            }
        }
    }
    pub fn clear_marks(&mut self, range: Range<usize>) {
        if range.is_empty() {
            self.typing_style = InlineStyle::default();
        } else {
            self.styles[range].fill(InlineStyle::default());
        }
    }
}
