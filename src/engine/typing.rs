//! Transformations de saisie explicites ; une insertion littérale ne les active jamais.
use super::*;
use crate::{
    document::{BlockKind, InlineStyle, Mark},
    session::Caret,
};

enum Rule {
    Block { start: usize, kind: BlockKind },
    Inline { start: usize, mark: Mark },
    Definition,
}

impl Engine {
    fn input_rule(&self, params: &InsertTextParams) -> Result<Option<Rule>, CommandError> {
        if !matches!(params.text.as_str(), " " | "*" | "_") {
            return Ok(None);
        }
        let selection = self.resolve(params.target)?;
        let range = selection.range();
        if !range.is_empty() {
            return Ok(None);
        }
        let view = &self.session.document.view;
        let start = view.line_ranges()[view.line_at(range.start)].start;
        let prefix: String = view
            .text
            .chars()
            .skip(start)
            .take(range.start - start)
            .collect();
        if params.text == " " {
            let kind = match prefix.as_str() {
                "=" => Some(BlockKind::Heading(1)),
                "==" => Some(BlockKind::Heading(2)),
                "===" => Some(BlockKind::Heading(3)),
                "-" => Some(BlockKind::Bullet),
                "+" | "1." => Some(BlockKind::Numbered),
                "/" => Some(BlockKind::Description),
                ">" => Some(BlockKind::Quote),
                _ => None,
            };
            if let Some(kind) = kind {
                return Ok(Some(Rule::Block { start, kind }));
            }
            if self
                .description_row(selection)
                .is_ok_and(|(_, separator)| separator == range.start)
                && prefix.ends_with(':')
            {
                return Ok(Some(Rule::Definition));
            }
        }
        if params.text == "*" || params.text == "_" {
            let delimiter = params.text.chars().next().unwrap();
            let chars: Vec<_> = prefix.chars().collect();
            if let Some(position) = chars.iter().rposition(|c| *c == delimiter)
                && position + 1 < chars.len()
                && (position == 0 || chars[position - 1].is_whitespace())
            {
                return Ok(Some(Rule::Inline {
                    start: start + position,
                    mark: if delimiter == '*' {
                        Mark::Bold
                    } else {
                        Mark::Italic
                    },
                }));
            }
        }
        Ok(None)
    }

    /// L’adaptateur intercepte uniquement une frappe qui déclenche une règle.
    /// Les autres événements gardent le tampon natif et son regroupement de frappe.
    pub(crate) fn transforms_input(&self, params: &InsertTextParams) -> bool {
        self.input_rule(params).is_ok_and(|rule| rule.is_some())
    }

    pub(super) fn transform_input(
        &mut self,
        params: &InsertTextParams,
    ) -> Result<bool, CommandError> {
        let Some(rule) = self.input_rule(params)? else {
            return Ok(false);
        };
        let selection = self.resolve(params.target)?;
        let at = selection.primary.index;
        let mut before = self.session.snapshot(selection);
        before.document.view.insert_text(&params.text, at);
        before.document.sync_source();
        let literal_end = at + params.text.chars().count();
        let literal_caret = Caret {
            index: literal_end,
            prefer_next_row: false,
        };
        before.selection = Selection {
            primary: literal_caret,
            secondary: literal_caret,
        };
        let mut view = self.session.document.view.clone();
        let at = match rule {
            Rule::Block { start, kind } => {
                view.delete_char_range(start..at);
                view.set_block(start..start, kind);
                start
            }
            Rule::Definition => {
                view.delete_char_range(at - 1..at);
                at
            }
            Rule::Inline { start, mark } => {
                view.delete_char_range(start..start + 1);
                view.set_mark(start..at - 1, mark, true);
                view.typing_style = InlineStyle::default();
                at - 1
            }
        };
        let caret = Caret {
            index: at,
            prefer_next_row: false,
        };
        self.commit_projection(
            self.session.target_guard(),
            before,
            view,
            Selection {
                primary: caret,
                secondary: caret,
            },
            EditGroup::Discrete,
        )?;
        Ok(true)
    }
}
