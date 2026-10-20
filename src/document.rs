//! Projection éditable du sous-ensemble Typst. Les indices publics sont des
//! indices de caractères Unicode (comme les curseurs egui), jamais des octets.
use std::ops::Range;

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
}

impl Default for Projection {
    fn default() -> Self {
        Self {
            text: String::new(),
            styles: vec![],
            blocks: vec![BlockKind::Paragraph],
            typing_style: InlineStyle::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    source: String,
    pub source_map: SourceMap,
    pub view: Projection,
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
            return Err("Le source Typst contient une erreur de syntaxe.".into());
        }
        let markup = tree
            .cast::<ast::Markup>()
            .ok_or("Document Typst illisible")?;
        let mut parser = Projector {
            source: Some(parsed.clone()),
            ..Default::default()
        };
        parser.markup(markup, &InlineStyle::default())?;
        parser.view.typing_style = InlineStyle::default();
        Ok(Self {
            source: source.to_owned(),
            source_map: SourceMap {
                characters: parser.origins,
                source_len: source.len(),
            },
            view: parser.view,
        })
    }

    pub fn demo() -> Self {
        Self::from_source(include_str!("../examples/courrier.typ"))
            .expect("courrier d’exemple valide")
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    /// Toute opération d'édition actualise le source qui sera compilé/exporté.
    pub fn sync_source(&mut self) {
        (self.source, self.source_map) = self.view.to_typst_mapped();
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
        for index in self.selected_lines(range) {
            self.blocks[index] = kind;
        }
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
            .filter(|(_, c)| *c != '\n')
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
                if c != '\n' {
                    apply(s);
                }
            }
        }
    }
    pub fn set_link(&mut self, range: Range<usize>, link: Option<String>) {
        for s in &mut self.styles[range] {
            s.link.clone_from(&link);
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
            if c == '\n' {
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
        let mut origins = Vec::with_capacity(self.len());
        let lines: Vec<_> = self.text.split('\n').collect();
        let ranges = self.line_ranges();
        for (index, text) in lines.iter().enumerate() {
            let kind = self.blocks[index];
            if index > 0 {
                let separator_start = result.len();
                let previous = self.blocks[index - 1];
                result.push_str(
                    if matches!(
                        (previous, kind),
                        (BlockKind::Bullet, BlockKind::Bullet)
                            | (BlockKind::Numbered, BlockKind::Numbered)
                    ) {
                        "\n"
                    } else {
                        "\n\n"
                    },
                );
                origins.push(separator_start..result.len());
            }
            match kind {
                BlockKind::Heading(level) => {
                    result.push_str(&"=".repeat(level as usize));
                    result.push(' ');
                }
                BlockKind::Bullet => result.push_str("- "),
                BlockKind::Numbered => result.push_str("+ "),
                BlockKind::Paragraph => {}
            }
            let characters: Vec<_> = text.chars().collect();
            let mut position = 0;
            while position < characters.len() {
                let style = &self.styles[ranges[index].start + position];
                let mut end = position + 1;
                while end < characters.len() && self.styles[ranges[index].start + end] == *style {
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
                for character in &characters[position..end] {
                    let start = result.len();
                    result.push_str(&escape_markup(&character.to_string()));
                    origins.push(start..result.len());
                }
                for _ in 0..usize::from(style.bold)
                    + usize::from(style.italic)
                    + usize::from(style.link.is_some())
                {
                    result.push(']');
                }
                position = end;
            }
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
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        let count = text.chars().count();
        let at = char_index.min(self.len());
        let line = self.line_at(at);
        let next_kind = match self.blocks[line] {
            BlockKind::Heading(_) => BlockKind::Paragraph,
            kind => kind,
        };
        let newlines = text.matches('\n').count();
        self.blocks
            .splice(line + 1..line + 1, std::iter::repeat_n(next_kind, newlines));
        self.text.insert_str(self.byte_index(at), &text);
        self.styles.splice(
            at..at,
            text.chars().map(|c| {
                if c == '\n' {
                    InlineStyle::default()
                } else {
                    self.typing_style.clone()
                }
            }),
        );
        count
    }
    fn delete_char_range(&mut self, range: Range<usize>) {
        let start = range.start.min(self.len());
        let end = range.end.min(self.len()).max(start);
        let line = self.line_at(start);
        let bytes = self.byte_index(start)..self.byte_index(end);
        let newlines = self.text[bytes.clone()].matches('\n').count();
        self.blocks.drain(line + 1..line + newlines + 1);
        self.text.replace_range(bytes, "");
        self.styles.drain(start..end);
    }
    fn replace_with(&mut self, text: &str) {
        *self = Self::default();
        self.insert_text(text, 0);
    }
}

pub fn typst_string(text: &str) -> String {
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
        if "\\#*_[]$`<>@=+-/~{}\"'".contains(c) {
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
    source: Option<typst_syntax::Source>,
    origin: Range<usize>,
    origins: Vec<Range<usize>>,
}
impl Projector {
    fn ensure_block(&mut self, kind: BlockKind) {
        if self.pending_break && !self.view.text.is_empty() {
            self.view.push("\n", &InlineStyle::default());
            self.origins.push(self.origin.start..self.origin.start);
        }
        if let Some(last) = self.view.blocks.last_mut() {
            *last = kind;
        }
        self.pending_break = false;
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
                    if !self.pending_break
                        && !self.view.text.is_empty()
                        && !self.view.text.ends_with([' ', '\n'])
                    {
                        self.append(" ", style);
                    }
                }
                Expr::Parbreak(_) => self.pending_break = true,
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
                        return Err("Seuls les titres de niveaux 1 à 3 sont pris en charge.".into());
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
                Expr::FuncCall(call) => self.call(call, style)?,
                Expr::Link(link) => {
                    let url = link.get().to_string();
                    let mut nested = style.clone();
                    nested.link = Some(url.clone());
                    self.append(&url, &nested);
                }
                _ => {
                    return Err(format!(
                        "Construction hors du sous-ensemble éditable : {:?}",
                        expr.to_untyped().kind()
                    ));
                }
            }
        }
        Ok(())
    }
    fn call(&mut self, call: ast::FuncCall<'_>, style: &InlineStyle) -> Result<(), String> {
        let Expr::Ident(callee) = call.callee() else {
            return Err("Appel calculé non pris en charge.".into());
        };
        let args: Vec<_> = call.args().items().collect();
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
                return Err(format!(
                    "Fonction ou signature non prise en charge : {}",
                    callee.as_str()
                ));
            }
        };
        self.inline_markup(body.body(), &nested)
    }
    fn inline_markup(
        &mut self,
        markup: ast::Markup<'_>,
        style: &InlineStyle,
    ) -> Result<(), String> {
        if markup.exprs().any(|expr| {
            matches!(
                expr,
                Expr::Heading(_) | Expr::ListItem(_) | Expr::EnumItem(_)
            )
        }) {
            return Err("Les blocs imbriqués ne sont pas pris en charge dans ce prototype.".into());
        }
        self.markup(markup, style)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Metadata {
    pub expediteur: String,
    pub fonction: String,
    pub service: String,
    pub email: String,
    pub destinataire: String,
    pub adresse: String,
    pub objet: String,
    pub reference: String,
    pub lieu: String,
    pub date: String,
}
impl Default for Metadata {
    fn default() -> Self {
        Self {
            expediteur: "Camille Martin".into(),
            fonction: "Responsable de la vie associative".into(),
            service: "Vie associative".into(),
            email: "camille.martin@valmont.example".into(),
            destinataire: "Service des équipements".into(),
            adresse: "12, place de la République\n35000 Valmont".into(),
            objet: "Demande de réservation d’une salle".into(),
            reference: "VA / 2026 / 042".into(),
            lieu: "Valmont".into(),
            date: "19 octobre 2026".into(),
        }
    }
}
impl Metadata {
    pub fn main_source(&self, compact: bool) -> String {
        let mut source = String::from("#import \"modele.typ\": courrier\n#show: courrier.with(\n");
        for (key, value) in [
            ("expediteur", &self.expediteur),
            ("fonction", &self.fonction),
            ("service", &self.service),
            ("email", &self.email),
            ("destinataire", &self.destinataire),
            ("adresse", &self.adresse),
            ("objet", &self.objet),
            ("reference", &self.reference),
            ("lieu", &self.lieu),
            ("date", &self.date),
        ] {
            source.push_str(&format!("  {key}: {},\n", typst_string(value)));
        }
        source.push_str(&format!(
            "  compact: {compact},\n)\n#include \"contenu.typ\"\n"
        ));
        source
    }
}
