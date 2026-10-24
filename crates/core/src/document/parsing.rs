//! Analyse du sous-ensemble Typst et construction de la projection.
use super::*;
use crate::errors::{CoreError, SourceError};
use typst_syntax::ast::{self, AstNode, Expr};
impl Document {
    pub fn from_source(source: &str) -> Result<Self, CoreError> {
        let parsed = typst_syntax::Source::detached(source);
        let tree = parsed.root();
        if tree.diagnosis().errors {
            return Err(CoreError::from(SourceError::SyntaxError));
        }
        let markup = tree
            .cast::<ast::Markup>()
            .ok_or(CoreError::from(SourceError::Unreadable))?;
        let mut parser = Projector {
            source: Some(parsed.clone()),
            ..Default::default()
        };
        parser.markup(markup, &InlineStyle::default())?;
        parser.finish();
        parser.view.typing_style = InlineStyle::default();
        let mut structure = Structure::default();
        structure.rebuild(&parser.view);
        Ok(Self {
            source: source.to_owned(),
            source_map: SourceMap {
                characters: parser.origins,
                source_len: source.len(),
            },
            view: parser.view,
            assets: Default::default(),
            structure,
        })
    }
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
        if self.pending_break
            && (!self.view.text.is_empty() || self.view.blocks[0].is_letter_note())
        {
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
    fn markup(&mut self, markup: ast::Markup<'_>, style: &InlineStyle) -> Result<(), CoreError> {
        for expr in markup.exprs() {
            self.origin = self
                .source
                .as_ref()
                .and_then(|s| s.find(expr.span()))
                .map_or(0..0, |node| node.range());
            match expr {
                Expr::ModuleImport(import)
                    if self.view.text.is_empty()
                        && import.to_untyped().full_text().trim()
                            == LETTER_NOTES_IMPORT.trim().trim_start_matches('#') => {}
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
                        return Err(CoreError::from(SourceError::HeadingDepth));
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
                    return Err(CoreError::from(SourceError::UnsupportedNode {
                        kind: (format!("{:?}", expr.to_untyped().kind())).to_string(),
                    }));
                }
            }
        }
        Ok(())
    }
    fn call(&mut self, call: ast::FuncCall<'_>, style: &InlineStyle) -> Result<(), CoreError> {
        let Expr::Ident(callee) = call.callee() else {
            return Err(CoreError::from(SourceError::ComputedCall));
        };
        let args: Vec<_> = call.args().items().collect();
        if matches!(callee.as_str(), "postscript" | "attachment") {
            return self.letter_note(callee.as_str(), &args, style);
        }
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
                return Err(CoreError::from(SourceError::UnsupportedFunction {
                    name: (callee.as_str()).to_string(),
                }));
            }
        };
        self.inline_markup(body.body(), &nested)
    }
    fn figure(&mut self, args: &[ast::Arg<'_>]) -> Result<(), CoreError> {
        let invalid = || CoreError::from(SourceError::ImageSignature);
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
    fn letter_note(
        &mut self,
        name: &str,
        args: &[ast::Arg<'_>],
        style: &InlineStyle,
    ) -> Result<(), CoreError> {
        let [ast::Arg::Pos(Expr::ContentBlock(body))] = args else {
            return Err(CoreError::from(SourceError::LetterNoteSignature));
        };
        let kind = if name == "postscript" {
            BlockKind::Postscript
        } else {
            BlockKind::Attachment
        };
        let mut note = Projector {
            source: self.source.clone(),
            ..Default::default()
        };
        note.inline_markup(body.body(), style)?;
        if kind == BlockKind::Attachment && note.view.blocks.len() != 1 {
            return Err(CoreError::from(SourceError::LetterNoteSignature));
        }
        note.view.blocks.fill(kind);
        self.pending_break =
            !self.view.text.is_empty() || self.view.blocks[0] != BlockKind::Paragraph;
        self.ensure_block(kind);
        self.extend(note);
        self.pending_break = true;
        Ok(())
    }
    fn quote(&mut self, args: &[ast::Arg<'_>], style: &InlineStyle) -> Result<(), CoreError> {
        let invalid = || CoreError::from(SourceError::QuoteSignature);
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
        self.view.line_ids.pop();
        self.view.line_ids.extend(other.view.line_ids);
        self.origins.extend(other.origins);
    }
    fn inline_markup(
        &mut self,
        markup: ast::Markup<'_>,
        style: &InlineStyle,
    ) -> Result<(), CoreError> {
        if markup.exprs().any(|expr| {
            matches!(
                expr,
                Expr::Heading(_) | Expr::ListItem(_) | Expr::EnumItem(_) | Expr::TermItem(_) | Expr::ModuleImport(_)
            )
            || matches!(expr, Expr::FuncCall(call) if matches!(call.callee(), Expr::Ident(name) if matches!(name.as_str(), "quote" | "figure" | "postscript" | "attachment")))
        }) {
            return Err(CoreError::from(SourceError::NestedBlocks));
        }
        self.markup(markup, style)
    }
}
