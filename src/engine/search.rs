//! Recherche sur la projection publiée ; aucune position graphique dans l’API.
use super::*;
use crate::{search::Search, session::Caret};
use serde_json::json;

impl Engine {
    fn find(&self, query: &str, case_sensitive: bool) -> Result<Search, CommandError> {
        let mut search = Search::default();
        search.query = query.into();
        search.case_sensitive = case_sensitive;
        search.refresh(self.session.revision, &self.session.document.view);
        if search.failed {
            return Err(CommandError::new(
                ErrorCode::InvalidParams,
                "search_too_large",
            ));
        }
        Ok(search)
    }

    pub(super) fn search_results(
        &self,
        params: SearchParams,
    ) -> Result<SearchResults, CommandError> {
        let search = self.find(&params.query, params.case_sensitive)?;
        let chars: Vec<_> = self.session.document.view.text.chars().collect();
        let mut matches = Vec::with_capacity(search.matches.len());
        for range in search.matches {
            let selection = Self::search_selection(range.clone());
            matches.push(SearchOccurrence {
                target: self.session.selection_target(selection)?,
                text: chars[range].iter().collect(),
            });
        }
        Ok(SearchResults {
            guard: self.session.target_guard(),
            matches,
        })
    }

    pub(super) fn check_search(&self, command: &Command) -> Result<(), CommandError> {
        let (query, case, replacement) = match command {
            Command::FindNext(p) | Command::FindPrevious(p) => (&p.query, p.case_sensitive, None),
            Command::ReplaceCurrent(p) => {
                // Résoudre la référence avant toute recherche ou mutation.
                self.resolve(p.occurrence.target.into())?;
                (&p.query, p.case_sensitive, Some(&p.replacement))
            }
            Command::ReplaceAll(p) => {
                self.check_guard(p.guard)?;
                (&p.query, p.case_sensitive, Some(&p.replacement))
            }
            _ => return Ok(()),
        };
        let mut search = self.find(query, case)?;
        if let Some(replacement) = replacement {
            search.replacement = replacement.clone();
            if !search.replacement_valid() {
                return Err(CommandError::new(
                    ErrorCode::InvalidParams,
                    "search_one_line",
                ));
            }
        }
        if let Command::ReplaceCurrent(p) = command {
            let range = self.resolve(p.occurrence.target.into())?.range();
            let actual: String = self
                .session
                .document
                .view
                .text
                .chars()
                .skip(range.start)
                .take(range.len())
                .collect();
            if search.selected(&range).is_none() || actual != p.occurrence.text {
                return Err(CommandError::new(ErrorCode::Document, "search_mismatch"));
            }
        }
        if search.matches.is_empty() {
            return Err(CommandError::new(
                ErrorCode::Unavailable,
                "search_no_results",
            ));
        }
        Ok(())
    }

    pub(super) fn navigate_search(
        &mut self,
        params: FindParams,
        backward: bool,
    ) -> Result<Value, CommandError> {
        let search = self.find(&params.query, params.case_sensitive)?;
        let selection = self.resolve(params.target)?;
        let index = search
            .next(selection.range(), backward)
            .expect("availability checked");
        self.select_resolved(Self::search_selection(search.matches[index].clone()));
        Ok(json!({"index": index, "count": search.matches.len()}))
    }

    pub(super) fn replace_search_current(
        &mut self,
        params: ReplaceCurrentParams,
    ) -> Result<Value, CommandError> {
        let selection = self.resolve(params.occurrence.target.into())?;
        let mut search = self.find(&params.query, params.case_sensitive)?;
        self.selection =
            self.session
                .replace_ranges(selection, &[selection.range()], &params.replacement)?;
        // Même motif déjà compilé avec succès : aucun échec possible à publier
        // après la transaction. Les résultats suivants sont calculés sur la nouvelle révision.
        search.refresh(self.session.revision, &self.session.document.view);
        if let Some(index) = search.next(self.selection.range(), false) {
            self.select_resolved(Self::search_selection(search.matches[index].clone()));
        }
        Ok(json!({"count": 1}))
    }

    pub(super) fn replace_search_all(
        &mut self,
        params: ReplaceAllParams,
    ) -> Result<Value, CommandError> {
        let search = self.find(&params.query, params.case_sensitive)?;
        let count = search.matches.len();
        self.selection =
            self.session
                .replace_ranges(self.selection, &search.matches, &params.replacement)?;
        Ok(json!({"count": count}))
    }

    fn search_selection(range: std::ops::Range<usize>) -> Selection {
        Selection {
            secondary: Caret {
                index: range.start,
                ..Default::default()
            },
            primary: Caret {
                index: range.end,
                ..Default::default()
            },
        }
    }
}
