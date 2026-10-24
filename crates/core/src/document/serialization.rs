//! Sérialisation Typst et carte des caractères, sans effet externe.
use super::*;
impl Projection {
    pub(super) fn push(&mut self, text: &str, style: &InlineStyle) {
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
        self.line_ids
            .extend(text.matches('\n').map(|_| LineId::new()));
    }
    pub fn to_typst_mapped(&self) -> (String, SourceMap) {
        let mut result = if self.blocks.iter().any(|kind| kind.is_letter_note()) {
            LETTER_NOTES_IMPORT.into()
        } else {
            String::new()
        };
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
            if kind.is_letter_note() {
                let end = if kind == BlockKind::Postscript {
                    let mut end = index + 1;
                    while self.blocks.get(end) == Some(&kind) {
                        end += 1;
                    }
                    end
                } else {
                    index + 1
                };
                result.push_str(if kind == BlockKind::Postscript {
                    "#postscript["
                } else {
                    "#attachment["
                });
                for line in index..end {
                    if line > index {
                        let start = result.len();
                        result.push('\n');
                        origins[ranges[line].start - 1] = start..result.len();
                    }
                    self.write_inline_line(line, &mut result, &mut origins);
                }
                result.push(']');
                index = end;
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
        if "\\#*_[]$`<>@=+-/~{}\"':".contains(c) {
            result.push('\\');
        }
        result.push(c);
    }
    result
}
