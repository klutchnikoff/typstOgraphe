//! Recherche littérale Unicode dans le contenu éditable, sans dépendance à egui.
use crate::document::{IMAGE_OBJECT, Projection};
use std::ops::Range;

#[derive(Default)]
pub struct Search {
    pub query: String,
    pub replacement: String,
    pub case_sensitive: bool,
    pub matches: Vec<Range<usize>>,
    pub failed: bool,
    cached: Option<(u64, String, bool)>,
}
impl Search {
    pub fn is_current(&self, revision: u64) -> bool {
        self.cached.as_ref().is_some_and(|(old, query, case)| {
            *old == revision && *query == self.query && *case == self.case_sensitive
        })
    }
    /// Cache graphique des résultats calculés par le moteur documentaire.
    pub fn cache_results(&mut self, revision: u64, matches: Vec<Range<usize>>, failed: bool) {
        self.cached = Some((revision, self.query.clone(), self.case_sensitive));
        self.matches = matches;
        self.failed = failed;
    }
    pub fn refresh(&mut self, revision: u64, view: &Projection) {
        if self.is_current(revision) {
            return;
        }
        self.cached = Some((revision, self.query.clone(), self.case_sensitive));
        self.matches.clear();
        self.failed = false;
        if self.query.is_empty() || self.query.chars().any(structural_character) {
            return;
        }
        // On réutilise la recherche Unicode de regex, mais jamais sa syntaxe :
        // tous les caractères saisis sont échappés, y compris .* et les crochets.
        let pattern = regex::RegexBuilder::new(&regex::escape(&self.query))
            .case_insensitive(!self.case_sensitive)
            .build();
        let Ok(pattern) = pattern else {
            self.failed = true;
            return;
        };
        let mut byte = 0;
        let mut character = 0;
        for found in pattern.find_iter(&view.text) {
            character += view.text[byte..found.start()].chars().count();
            let start = character;
            character += found.as_str().chars().count();
            byte = found.end();
            // Les paramètres privés et les séparateurs de structures ne sont
            // pas recherchables. Les styles n’interrompent pas un résultat.
            if !found.as_str().chars().any(structural_character) {
                self.matches.push(start..character);
            }
        }
    }
    pub fn selected(&self, selection: &Range<usize>) -> Option<usize> {
        self.matches
            .binary_search_by_key(&selection.start, |range| range.start)
            .ok()
            .filter(|index| self.matches[*index] == *selection)
    }
    pub fn next(&self, selection: Range<usize>, backward: bool) -> Option<usize> {
        if self.matches.is_empty() {
            return None;
        }
        if let Some(index) = self.selected(&selection) {
            return Some(if backward {
                (index + self.matches.len() - 1) % self.matches.len()
            } else {
                (index + 1) % self.matches.len()
            });
        }
        if backward {
            Some(
                self.matches
                    .iter()
                    .rposition(|range| range.start < selection.start)
                    .unwrap_or(self.matches.len() - 1),
            )
        } else {
            Some(
                self.matches
                    .iter()
                    .position(|range| range.start >= selection.end)
                    .unwrap_or(0),
            )
        }
    }
    pub fn replacement_valid(&self) -> bool {
        Self::valid_replacement(&self.replacement)
    }
    pub(crate) fn valid_replacement(replacement: &str) -> bool {
        !replacement.chars().any(structural_character)
    }
}
fn structural_character(character: char) -> bool {
    character.is_control() || matches!(character, IMAGE_OBJECT | '\u{2028}' | '\u{2029}')
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::document::Document;

    #[test]
    fn literal_search_uses_unicode_case_and_character_positions_across_styles() {
        let document =
            Document::from_source("Été *ÉTÉ* ete. Un .\\* littéral et un _été_.").unwrap();
        let mut search = Search {
            query: "été".into(),
            ..Default::default()
        };
        search.refresh(1, &document.view);
        assert_eq!(search.matches, vec![0..3, 4..7, 34..37]);
        search.case_sensitive = true;
        search.refresh(1, &document.view);
        assert_eq!(search.matches, vec![34..37]);
        search.query = ".*".into();
        search.refresh(1, &document.view);
        assert_eq!(search.matches.len(), 1);
        search.query = "Un .* littéral".into();
        search.refresh(1, &document.view);
        assert_eq!(search.matches, vec![13..27]);
        search.query.clear();
        search.refresh(1, &document.view);
        assert!(search.matches.is_empty());
    }

    #[test]
    fn navigation_wraps_and_search_includes_annotations_but_excludes_structure_and_private_fields()
    {
        let document = Document::from_source("= Mot\n\n/ Mot: Mot\n\n#quote(block: true, attribution: [Mot])[Mot]\n\n#figure(image(\"images/mot.svg\", alt: \"Mot\"), caption: [Mot])").unwrap();
        let mut search = Search {
            query: "mot".into(),
            ..Default::default()
        };
        search.refresh(1, &document.view);
        assert_eq!(search.matches.len(), 6);
        assert_eq!(search.next(0..0, false), Some(0));
        assert_eq!(search.next(search.matches[5].clone(), false), Some(0));
        assert_eq!(search.next(search.matches[0].clone(), true), Some(5));
        assert_eq!(search.selected(&search.matches[2]), Some(2));
        for query in ["Mot\nMot", "Mot\tMot", "\u{fffc}"] {
            search.query = query.into();
            search.refresh(1, &document.view);
            assert!(search.matches.is_empty());
        }
    }
}
