//! Projection éditable du sous-ensemble Typst. Les indices publics sont des
//! indices de caractères Unicode (comme les curseurs egui), jamais des octets.
use std::{collections::BTreeMap, ops::Range};

pub const IMAGE_OBJECT: char = '\u{fffc}';

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageBlock {
    pub path: String,
    pub alt: String,
}

use eframe::egui::TextBuffer;
use serde::{Deserialize, Serialize};
use typst_syntax::ast::{self, AstNode, Expr};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InlineStyle {
    pub bold: bool,
    pub italic: bool,
    pub link: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BlockKind {
    #[default]
    Paragraph,
    Heading(u8),
    Bullet,
    Numbered,
    Description,
    Quote,
    QuoteAttribution,
    Image(u64),
}
impl BlockKind {
    pub fn is_list(self) -> bool {
        matches!(self, Self::Bullet | Self::Numbered | Self::Description)
    }
    pub fn public_kind(self) -> Self {
        match self {
            Self::QuoteAttribution => Self::Quote,
            Self::Image(_) => Self::Paragraph,
            _ => self,
        }
    }
}

pub struct QuoteGroup {
    pub body: Range<usize>,
    pub attribution: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Bold,
    Italic,
}

/// Une seule chaîne visible : la sélection n'est jamais coupée aux frontières
/// des paragraphes. Les styles sont une projection du source, non un format disque.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Projection {
    pub text: String,
    pub styles: Vec<InlineStyle>,
    pub blocks: Vec<BlockKind>,
    pub typing_style: InlineStyle,
    pub images: BTreeMap<u64, ImageBlock>,
}

impl Default for Projection {
    fn default() -> Self {
        Self {
            text: String::new(),
            styles: vec![],
            blocks: vec![BlockKind::Paragraph],
            typing_style: InlineStyle::default(),
            images: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    source: String,
    pub source_map: SourceMap,
    pub view: Projection,
    pub assets: crate::assets::Assets,
}

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

impl Document {
    pub fn from_source(source: &str) -> Result<Self, String> {
        let parsed = typst_syntax::Source::detached(source);
        let tree = parsed.root();
        if tree.diagnosis().errors {
            return Err(crate::tr!("source-syntax-error"));
        }
        let markup = tree
            .cast::<ast::Markup>()
            .ok_or(crate::tr!("source-unreadable"))?;
        let mut parser = Projector {
            source: Some(parsed.clone()),
            ..Default::default()
        };
        parser.markup(markup, &InlineStyle::default())?;
        parser.finish();
        parser.view.typing_style = InlineStyle::default();
        Ok(Self {
            source: source.to_owned(),
            source_map: SourceMap {
                characters: parser.origins,
                source_len: source.len(),
            },
            view: parser.view,
            assets: Default::default(),
        })
    }

    #[cfg(test)]
    pub fn demo() -> Self {
        Self::from_source(include_str!("../examples/courrier.typ"))
            .expect("courrier d’exemple valide")
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    /// Toute opération d'édition actualise le source qui sera compilé/exporté.
    /// Renvoie les séparateurs dans l’ordre des insertions afin que l’UI puisse
    /// conserver la sélection en créant les zones de description et d’attribution.
    pub fn sync_source(&mut self) -> Vec<usize> {
        let mut inserted = self.view.ensure_description_separators();
        inserted.extend(self.view.ensure_quote_attributions());
        inserted.extend(self.view.ensure_list_boundaries());
        (self.source, self.source_map) = self.view.to_typst_mapped();
        inserted
    }
}

impl Projection {
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
    pub fn style_at(&self, position: usize) -> InlineStyle {
        self.styles
            .get(position)
            .or_else(|| self.styles.last())
            .cloned()
            .unwrap_or_default()
    }
    pub fn set_block(&mut self, range: Range<usize>, kind: BlockKind) {
        let lines = self.line_ranges();
        let selected = !range.is_empty();
        for index in self.selected_lines(range) {
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
        let id = self.images.keys().next_back().copied().unwrap_or(0) + 1;
        let at = at.min(self.len());
        let line = self.line_at(at);
        let start = self.line_ranges()[line].start;
        let prefix = if at == start { "" } else { "\n" };
        let text = format!("{prefix}{IMAGE_OBJECT}\n");
        self.insert_text(&text, at);
        let position = at + prefix.len();
        let line = self.line_at(position);
        self.blocks[line] = BlockKind::Image(id);
        if !matches!(self.blocks[line + 1], BlockKind::Image(_)) {
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
    fn clean_images(&mut self) {
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
    fn ensure_quote_attributions(&mut self) -> Vec<usize> {
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
    fn ensure_description_separators(&mut self) -> Vec<usize> {
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
    fn ensure_list_boundaries(&mut self) -> Vec<usize> {
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
    fn push(&mut self, text: &str, style: &InlineStyle) {
        self.text.push_str(text);
        self.styles.extend(text.chars().map(|c| {
            if matches!(c, '\n' | '\t') {
                InlineStyle::default()
            } else {
                style.clone()
            }
        }));
        self.blocks.extend(std::iter::repeat_n(
            BlockKind::Paragraph,
            text.matches('\n').count(),
        ));
    }
    pub fn to_typst_mapped(&self) -> (String, SourceMap) {
        let mut result = String::new();
        let mut origins = vec![0..0; self.len()];
        let ranges = self.line_ranges();
        let mut index = 0;
        while index < ranges.len() {
            let kind = self.blocks[index];
            if index > 0 {
                let start = result.len();
                result.push('\n');
                origins[ranges[index].start - 1] = start..result.len();
            }
            if let BlockKind::Image(id) = kind {
                let image = &self.images[&id];
                result.push_str("#figure(");
                let start = result.len();
                result.push_str(&format!(
                    "image({}, alt: {}),",
                    typst_string(&image.path),
                    if image.alt.is_empty() {
                        "none".into()
                    } else {
                        typst_string(&image.alt)
                    }
                ));
                origins[ranges[index].start] = start..result.len();
                if ranges[index].len() > 1 {
                    result.push_str(" caption: [");
                    self.write_inline_line(index, &mut result, &mut origins);
                    result.push_str("],");
                }
                result.push(')');
                index += 1;
                continue;
            }
            if kind == BlockKind::Quote {
                let mut end = index + 1;
                while self.blocks.get(end) == Some(&BlockKind::Quote) {
                    end += 1;
                }
                let attribution =
                    (self.blocks.get(end) == Some(&BlockKind::QuoteAttribution)).then_some(end);
                result.push_str("#quote(block: true");
                if let Some(line) = attribution
                    && !ranges[line].is_empty()
                {
                    result.push_str(", attribution: [");
                    self.write_inline_line(line, &mut result, &mut origins);
                    result.push(']');
                }
                result.push_str(")[");
                for line in index..end {
                    if line > index {
                        let start = result.len();
                        result.push('\n');
                        origins[ranges[line].start - 1] = start..result.len();
                    }
                    self.write_inline_line(line, &mut result, &mut origins);
                }
                result.push(']');
                if let Some(line) = attribution {
                    origins[ranges[line].start - 1] = result.len()..result.len();
                }
                index = end + usize::from(attribution.is_some());
                continue;
            }
            match kind {
                BlockKind::Heading(level) => {
                    result.push_str(&"=".repeat(level as usize));
                    result.push(' ');
                }
                BlockKind::Bullet => result.push_str("- "),
                BlockKind::Numbered => result.push_str("+ "),
                BlockKind::Description => result.push_str("/ "),
                _ => {}
            }
            self.write_inline_line(index, &mut result, &mut origins);
            index += 1;
        }
        let source_len = result.len();
        (
            result,
            SourceMap {
                characters: origins,
                source_len,
            },
        )
    }
    fn write_inline_line(&self, index: usize, result: &mut String, origins: &mut [Range<usize>]) {
        let ranges = self.line_ranges();
        let text =
            &self.text[self.byte_index(ranges[index].start)..self.byte_index(ranges[index].end)];
        let kind = self.blocks[index];
        let characters: Vec<_> = text.chars().collect();
        let description_separator = (kind == BlockKind::Description)
            .then(|| characters.iter().position(|c| *c == '\t'))
            .flatten();
        let mut position = usize::from(matches!(kind, BlockKind::Image(_)));
        while position < characters.len() {
            if Some(position) == description_separator {
                let start = result.len();
                result.push_str(": ");
                origins[ranges[index].start + position] = start..result.len();
                position += 1;
                continue;
            }
            let style = &self.styles[ranges[index].start + position];
            let mut end = position + 1;
            while end < characters.len()
                && Some(end) != description_separator
                && self.styles[ranges[index].start + end] == *style
            {
                end += 1;
            }
            if let Some(url) = &style.link {
                result.push_str(&format!("#link({})[", typst_string(url)));
            }
            if style.bold {
                result.push_str("#strong[");
            }
            if style.italic {
                result.push_str("#emph[");
            }
            for (offset, character) in characters[position..end].iter().enumerate() {
                let start = result.len();
                // Un point après un appel (#strong[...].texte) serait analysé
                // comme un accès à un champ, et doit rester du texte littéral.
                if *character == '.' && result.ends_with(']') {
                    result.push('\\');
                }
                result.push_str(&escape_markup(&character.to_string()));
                origins[ranges[index].start + position + offset] = start..result.len();
            }
            for _ in 0..usize::from(style.bold)
                + usize::from(style.italic)
                + usize::from(style.link.is_some())
            {
                result.push(']');
            }
            position = end;
        }
        if kind == BlockKind::Description && !characters.contains(&'\t') {
            result.push_str(": ");
        }
    }
}

impl TextBuffer for Projection {
    fn type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }
    fn is_mutable(&self) -> bool {
        true
    }
    fn as_str(&self) -> &str {
        &self.text
    }
    fn insert_text(&mut self, text: &str, char_index: usize) -> usize {
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
    fn delete_char_range(&mut self, range: Range<usize>) {
        let start = range.start.min(self.len());
        let end = range.end.min(self.len()).max(start);
        let line = self.line_at(start);
        let bytes = self.byte_index(start)..self.byte_index(end);
        let newlines = self.text[bytes.clone()].matches('\n').count();
        if start == self.line_ranges()[line].start && newlines > 0 {
            self.blocks[line] = self.blocks[line + newlines];
        }
        self.blocks.drain(line + 1..line + newlines + 1);
        self.text.replace_range(bytes, "");
        self.styles.drain(start..end);
        self.clean_images();
    }
    fn replace_with(&mut self, text: &str) {
        *self = Self::default();
        self.insert_text(text, 0);
    }
}

pub(crate) fn typst_string(text: &str) -> String {
    // Les chaînes JSON partagent les échappements usuels avec les chaînes Typst.
    // Les contrôles moins courants sont écrits avec la syntaxe Unicode de Typst.
    let mut result = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c.is_control() => result.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => result.push(c),
        }
    }
    result.push('"');
    result
}

fn escape_markup(text: &str) -> String {
    let mut result = String::new();
    for c in text.chars() {
        if "\\#*_[]$`<>@=+-/~{}\"':".contains(c) {
            result.push('\\');
        }
        result.push(c);
    }
    result
}

#[derive(Default)]
struct Projector {
    view: Projection,
    pending_break: bool,
    pending_newlines: usize,
    source: Option<typst_syntax::Source>,
    origin: Range<usize>,
    origins: Vec<Range<usize>>,
}
impl Projector {
    fn ensure_block(&mut self, kind: BlockKind) {
        if self.pending_break && !self.view.text.is_empty() {
            let count = self.pending_newlines.max(1);
            self.view.push(&"\n".repeat(count), &InlineStyle::default());
            self.origins.extend(std::iter::repeat_n(
                self.origin.start..self.origin.start,
                count,
            ));
        }
        if let Some(last) = self.view.blocks.last_mut() {
            *last = kind;
        }
        self.pending_break = false;
        self.pending_newlines = 0;
    }
    fn finish(&mut self) {
        if self.pending_newlines > 0 {
            self.ensure_block(BlockKind::Paragraph);
        }
    }
    fn append(&mut self, text: &str, style: &InlineStyle) {
        if self.pending_break {
            self.ensure_block(BlockKind::Paragraph);
        }
        self.view.push(text, style);
        let exact = self
            .source
            .as_ref()
            .and_then(|s| s.text().get(self.origin.clone()))
            == Some(text);
        for (offset, c) in text.char_indices() {
            self.origins.push(if exact {
                self.origin.start + offset..self.origin.start + offset + c.len_utf8()
            } else {
                self.origin.clone()
            });
        }
    }
    fn markup(&mut self, markup: ast::Markup<'_>, style: &InlineStyle) -> Result<(), String> {
        for expr in markup.exprs() {
            self.origin = self
                .source
                .as_ref()
                .and_then(|s| s.find(expr.span()))
                .map_or(0..0, |node| node.range());
            match expr {
                Expr::Text(text) => self.append(text.get(), style),
                Expr::Escape(escaped) => self.append(&escaped.get().to_string(), style),
                Expr::Shorthand(short) => self.append(&short.get().to_string(), style),
                Expr::SmartQuote(quote) => {
                    self.append(if quote.double() { "\"" } else { "'" }, style)
                }
                Expr::Space(_) => {
                    let newline = self
                        .source
                        .as_ref()
                        .and_then(|source| source.text().get(self.origin.clone()))
                        .is_some_and(|space| space.contains('\n'));
                    if newline && self.pending_break {
                        self.pending_newlines = self.pending_newlines.max(1);
                    } else if newline
                        && !self.view.text.is_empty()
                        && self.view.blocks.last() == Some(&BlockKind::Paragraph)
                    {
                        self.append("\n", &InlineStyle::default());
                    } else if !self.pending_break
                        && !self.view.text.is_empty()
                        && !self.view.text.ends_with([' ', '\n'])
                    {
                        self.append(" ", style);
                    }
                }
                Expr::Parbreak(_) => {
                    self.pending_break = true;
                    self.pending_newlines = 2;
                }
                Expr::Strong(strong) => {
                    let mut nested = style.clone();
                    nested.bold = true;
                    self.inline_markup(strong.body(), &nested)?;
                }
                Expr::Emph(emph) => {
                    let mut nested = style.clone();
                    nested.italic = true;
                    self.inline_markup(emph.body(), &nested)?;
                }
                Expr::Heading(heading) => {
                    if heading.depth().get() > 3 {
                        return Err(crate::tr!("source-heading-depth"));
                    }
                    self.pending_break = !self.view.text.is_empty();
                    self.ensure_block(BlockKind::Heading(heading.depth().get() as u8));
                    self.inline_markup(heading.body(), style)?;
                    self.pending_break = true;
                }
                Expr::ListItem(item) => {
                    self.pending_break = !self.view.text.is_empty();
                    self.ensure_block(BlockKind::Bullet);
                    self.inline_markup(item.body(), style)?;
                    self.pending_break = true;
                }
                Expr::EnumItem(item) => {
                    self.pending_break = !self.view.text.is_empty();
                    self.ensure_block(BlockKind::Numbered);
                    self.inline_markup(item.body(), style)?;
                    self.pending_break = true;
                }
                Expr::TermItem(item) => {
                    let separator = item
                        .to_untyped()
                        .children()
                        .find(|node| node.kind() == typst_syntax::SyntaxKind::Colon)
                        .and_then(|node| self.source.as_ref()?.find(node.span()))
                        .map_or(self.origin.clone(), |node| node.range());
                    self.pending_break = !self.view.text.is_empty();
                    self.ensure_block(BlockKind::Description);
                    self.inline_markup(item.term(), style)?;
                    // Une tabulation sépare les deux zones éditables sans
                    // exposer la syntaxe Typst. Les deux restent sélectionnables.
                    self.origin = separator;
                    self.append("\t", &InlineStyle::default());
                    self.inline_markup(item.description(), style)?;
                    self.pending_break = true;
                }
                Expr::FuncCall(call) => self.call(call, style)?,
                Expr::Link(link) => {
                    let url = link.get().to_string();
                    let mut nested = style.clone();
                    nested.link = Some(url.clone());
                    self.append(&url, &nested);
                }
                _ => {
                    return Err(crate::tr!(
                        "source-unsupported-node",
                        kind = format!("{:?}", expr.to_untyped().kind())
                    ));
                }
            }
        }
        Ok(())
    }
    fn call(&mut self, call: ast::FuncCall<'_>, style: &InlineStyle) -> Result<(), String> {
        let Expr::Ident(callee) = call.callee() else {
            return Err(crate::tr!("source-computed-call"));
        };
        let args: Vec<_> = call.args().items().collect();
        if callee.as_str() == "figure" {
            return self.figure(&args);
        }
        if callee.as_str() == "quote" {
            return self.quote(&args, style);
        }
        let mut nested = style.clone();
        let body = match (callee.as_str(), args.as_slice()) {
            ("strong", [ast::Arg::Pos(Expr::ContentBlock(body))]) => {
                nested.bold = true;
                *body
            }
            ("emph", [ast::Arg::Pos(Expr::ContentBlock(body))]) => {
                nested.italic = true;
                *body
            }
            (
                "link",
                [
                    ast::Arg::Pos(Expr::Str(url)),
                    ast::Arg::Pos(Expr::ContentBlock(body)),
                ],
            ) => {
                nested.link = Some(url.get().to_string());
                *body
            }
            _ => {
                return Err(crate::tr!(
                    "source-unsupported-function",
                    name = callee.as_str()
                ));
            }
        };
        self.inline_markup(body.body(), &nested)
    }
    fn figure(&mut self, args: &[ast::Arg<'_>]) -> Result<(), String> {
        let invalid = || crate::tr!("source-image-signature");
        let mut image = None;
        let mut caption = None;
        for arg in args {
            match arg {
                ast::Arg::Pos(Expr::FuncCall(call)) if image.is_none() => image = Some(*call),
                ast::Arg::Named(named)
                    if named.name().as_str() == "caption" && caption.is_none() =>
                {
                    let Expr::ContentBlock(content) = named.expr() else {
                        return Err(invalid());
                    };
                    caption = Some(content);
                }
                _ => return Err(invalid()),
            }
        }
        let image = image.ok_or_else(invalid)?;
        if !matches!(image.callee(), Expr::Ident(name) if name.as_str() == "image") {
            return Err(invalid());
        }
        let mut path = None;
        let mut alt = None;
        for arg in image.args().items() {
            match arg {
                ast::Arg::Pos(Expr::Str(value)) if path.is_none() => {
                    path = Some(value.get().to_string())
                }
                ast::Arg::Named(named) if named.name().as_str() == "alt" && alt.is_none() => {
                    alt = Some(match named.expr() {
                        Expr::Str(value) => value.get().to_string(),
                        Expr::None(_) => String::new(),
                        _ => return Err(invalid()),
                    });
                }
                _ => return Err(invalid()),
            }
        }
        let path = path.ok_or_else(invalid)?;
        if !crate::assets::valid_path(&path) {
            return Err(invalid());
        }
        let mut projected = Projector {
            source: self.source.clone(),
            ..Default::default()
        };
        if let Some(caption) = caption {
            projected.inline_markup(caption.body(), &InlineStyle::default())?;
        }
        if projected.view.blocks.len() != 1 {
            return Err(invalid());
        }
        let id = self.view.images.keys().next_back().copied().unwrap_or(0) + 1;
        self.pending_break = !self.view.text.is_empty();
        self.ensure_block(BlockKind::Image(id));
        let figure_origin = self.origin.clone();
        self.origin = self
            .source
            .as_ref()
            .and_then(|s| s.find(image.span()))
            .map_or(figure_origin, |node| node.range());
        self.append(&IMAGE_OBJECT.to_string(), &InlineStyle::default());
        self.view.images.insert(
            id,
            ImageBlock {
                path,
                alt: alt.unwrap_or_default(),
            },
        );
        projected.view.blocks[0] = BlockKind::Image(id);
        self.extend(projected);
        self.pending_break = true;
        Ok(())
    }
    fn quote(&mut self, args: &[ast::Arg<'_>], style: &InlineStyle) -> Result<(), String> {
        let invalid = || crate::tr!("source-quote-signature");
        let mut block = false;
        let mut body = None;
        let mut attribution = None;
        let mut names = std::collections::HashSet::new();
        for arg in args {
            match arg {
                ast::Arg::Pos(Expr::ContentBlock(content)) if body.is_none() => {
                    body = Some(*content)
                }
                ast::Arg::Named(named) if names.insert(named.name().as_str()) => {
                    match (named.name().as_str(), named.expr()) {
                        ("block", Expr::Bool(value)) if value.get() => block = true,
                        ("attribution", Expr::ContentBlock(content)) => attribution = Some(content),
                        _ => return Err(invalid()),
                    }
                }
                _ => return Err(invalid()),
            }
        }
        if !block {
            return Err(invalid());
        }
        let body = body.ok_or_else(invalid)?;
        let mut quoted = Projector {
            source: self.source.clone(),
            ..Default::default()
        };
        quoted.inline_markup(body.body(), style)?;
        quoted.view.blocks.fill(BlockKind::Quote);
        self.pending_break = !self.view.blocks.is_empty()
            && (!self.view.text.is_empty() || self.view.blocks[0] != BlockKind::Paragraph);
        self.ensure_block(BlockKind::Quote);
        self.extend(quoted);
        self.view.push("\n", &InlineStyle::default());
        self.origins.push(self.origin.start..self.origin.start);
        *self.view.blocks.last_mut().unwrap() = BlockKind::QuoteAttribution;
        if let Some(content) = attribution {
            let mut author = Projector {
                source: self.source.clone(),
                ..Default::default()
            };
            author.inline_markup(content.body(), &InlineStyle::default())?;
            if author.view.blocks.len() != 1 {
                return Err(invalid());
            }
            author.view.blocks[0] = BlockKind::QuoteAttribution;
            self.extend(author);
        }
        self.pending_break = true;
        Ok(())
    }
    fn extend(&mut self, other: Projector) {
        self.view.text.push_str(&other.view.text);
        self.view.styles.extend(other.view.styles);
        self.view.blocks.pop();
        self.view.blocks.extend(other.view.blocks);
        self.origins.extend(other.origins);
    }
    fn inline_markup(
        &mut self,
        markup: ast::Markup<'_>,
        style: &InlineStyle,
    ) -> Result<(), String> {
        if markup.exprs().any(|expr| {
            matches!(
                expr,
                Expr::Heading(_) | Expr::ListItem(_) | Expr::EnumItem(_) | Expr::TermItem(_)
            )
            || matches!(expr, Expr::FuncCall(call) if matches!(call.callee(), Expr::Ident(name) if matches!(name.as_str(), "quote" | "figure")))
        }) {
            return Err(crate::tr!("source-nested-blocks"));
        }
        self.markup(markup, style)
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PaperFormat {
    A3,
    #[default]
    A4,
    A5,
    #[serde(rename = "us-letter")]
    Letter,
    #[serde(rename = "us-legal")]
    Legal,
}
impl PaperFormat {
    pub const ALL: [Self; 5] = [Self::A3, Self::A4, Self::A5, Self::Letter, Self::Legal];
    pub fn typst_name(self) -> &'static str {
        match self {
            Self::A3 => "a3",
            Self::A4 => "a4",
            Self::A5 => "a5",
            Self::Letter => "us-letter",
            Self::Legal => "us-legal",
        }
    }
    pub fn label(self) -> String {
        match self {
            Self::A3 => "A3".into(),
            Self::A4 => "A4".into(),
            Self::A5 => "A5".into(),
            Self::Letter => crate::tr!("paper-letter"),
            Self::Legal => crate::tr!("paper-legal"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum DocumentLanguage {
    #[default]
    #[serde(rename = "fr")]
    French,
    #[serde(rename = "en")]
    English,
    #[serde(rename = "de")]
    German,
    #[serde(rename = "es")]
    Spanish,
    #[serde(rename = "it")]
    Italian,
    #[serde(rename = "pt")]
    Portuguese,
    #[serde(rename = "nl")]
    Dutch,
}
impl DocumentLanguage {
    pub const ALL: [Self; 7] = [
        Self::French,
        Self::English,
        Self::German,
        Self::Spanish,
        Self::Italian,
        Self::Portuguese,
        Self::Dutch,
    ];
    pub fn code(self) -> &'static str {
        match self {
            Self::French => "fr",
            Self::English => "en",
            Self::German => "de",
            Self::Spanish => "es",
            Self::Italian => "it",
            Self::Portuguese => "pt",
            Self::Dutch => "nl",
        }
    }
    pub fn label(self) -> String {
        match self {
            Self::French => crate::tr!("language-french"),
            Self::English => crate::tr!("language-english"),
            Self::German => crate::tr!("language-german"),
            Self::Spanish => crate::tr!("language-spanish"),
            Self::Italian => crate::tr!("language-italian"),
            Self::Portuguese => crate::tr!("language-portuguese"),
            Self::Dutch => crate::tr!("language-dutch"),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocumentOptions {
    pub paper: PaperFormat,
    pub landscape: bool,
    pub language: DocumentLanguage,
    /// None laisse au modèle le choix du marqueur selon sa charte et la langue.
    pub bullet_marker: Option<String>,
}

pub fn valid_bullet_marker(marker: &str) -> bool {
    let mut chars = marker.chars();
    chars
        .next()
        .is_some_and(|c| !c.is_whitespace() && !c.is_control())
        && chars.next().is_none()
}
