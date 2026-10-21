//! Comptage du texte éditable, indépendant du modèle et du rendu Typst.
use crate::document::{IMAGE_OBJECT, Projection};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextStatistics {
    pub words: usize,
    pub signs: usize,
    pub signs_without_spaces: usize,
}

impl TextStatistics {
    pub fn count(text: &str) -> Self {
        let mut stats = Self {
            words: text.unicode_words().count(),
            ..Default::default()
        };
        for grapheme in text.graphemes(true) {
            // Les tabulations des descriptions et les objets images sont des
            // marqueurs structurels. Les retours ne sont pas des signes rédigés.
            if grapheme
                .chars()
                .all(|c| c.is_control() || matches!(c, IMAGE_OBJECT | '\u{2028}' | '\u{2029}'))
            {
                continue;
            }
            stats.signs += 1;
            if !grapheme.chars().all(char::is_whitespace) {
                stats.signs_without_spaces += 1;
            }
        }
        stats
    }
}

/// Données dérivées en mémoire : les totaux ne sont recomptés qu’après édition,
/// la sélection seulement quand elle change. Aucun changement du format Typst.
#[derive(Default)]
pub struct Counter {
    revision: Option<u64>,
    total: TextStatistics,
    selected: Option<(Range<usize>, TextStatistics)>,
}

impl Counter {
    pub fn counts(
        &mut self,
        revision: u64,
        view: &Projection,
        range: Range<usize>,
    ) -> (TextStatistics, Option<TextStatistics>) {
        if self.revision != Some(revision) {
            self.total = TextStatistics::count(&view.text);
            self.selected = None;
            self.revision = Some(revision);
        }
        if range.is_empty() {
            self.selected = None;
        } else if !self
            .selected
            .as_ref()
            .is_some_and(|(previous, _)| *previous == range)
        {
            let text = &view.text[view.byte_index(range.start)..view.byte_index(range.end)];
            self.selected = Some((range, TextStatistics::count(text)));
        }
        (self.total, self.selected.as_ref().map(|(_, stats)| *stats))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;

    #[test]
    fn unicode_counts_graphemes_and_words_with_explicit_punctuation_conventions() {
        for (text, words, signs, without_spaces) in [
            ("", 0, 0, 0),
            ("é", 1, 1, 1),
            ("e\u{301}", 1, 1, 1),
            ("👩🏽‍💻 🇫🇷", 0, 3, 2),
            ("l’été porte-monnaie 12,5", 4, 24, 22),
            ("A\u{a0}B C", 3, 5, 3),
            ("12\u{202f}500", 1, 6, 5),
            ("Un\tdeux\r\ntrois\u{2028}quatre", 4, 17, 17),
            ("\n\t\u{fffc}\n", 0, 0, 0),
        ] {
            assert_eq!(
                TextStatistics::count(text),
                TextStatistics {
                    words,
                    signs,
                    signs_without_spaces: without_spaces
                },
                "{text:?}"
            );
        }
    }

    #[test]
    fn counts_only_authored_text_and_updates_selection_and_document_revisions() {
        let mut document = Document::from_source("= Été\n\n#figure(image(\"images/parc.svg\", alt: \"Ce texte alternatif ne compte pas\"), caption: [Le parc.])\n\n#quote(block: true, attribution: [Élodie])[Une idée.]\n\n/ Échéance: Vendredi").unwrap();
        let mut counter = Counter::default();
        let expected =
            TextStatistics::count("Été\nLe parc.\nUne idée.\nÉlodie\nÉchéance\tVendredi");
        let (total, selected) = counter.counts(1, &document.view, 0..0);
        assert_eq!(total, expected);
        assert_eq!(selected, None);
        assert_eq!(
            counter.counts(1, &document.view, 0..3).1,
            Some(TextStatistics {
                words: 1,
                signs: 3,
                signs_without_spaces: 3
            })
        );
        let end = document.view.len();
        assert_eq!(counter.counts(1, &document.view, 0..end).1, Some(total));
        document.view.insert_text(" Très bien.", end);
        let updated = counter.counts(2, &document.view, 0..0);
        assert_eq!(updated.0.words, total.words + 2);
        assert_eq!(updated.0.signs, total.signs + 11);
        assert_eq!(
            updated.0.signs_without_spaces,
            total.signs_without_spaces + 9
        );
        assert_eq!(updated.1, None);
    }
}
