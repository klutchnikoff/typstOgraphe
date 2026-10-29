//! Comptage du texte éditable, indépendant du modèle et du rendu Typst.
const IMAGE_OBJECT: char = '\u{fffc}';
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

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
}
