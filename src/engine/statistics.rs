//! Lectures dérivées de la projection publiée ; jamais de la préédition IME.
use super::*;
use crate::statistics::{Counter, TextStatistics};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StatisticsParams {
    /// Absente : document seul. Current : sélection détenue par le moteur.
    #[serde(default)]
    pub target: Option<TextTarget>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StatisticsResult {
    pub guard: TargetGuard,
    pub document: TextStatistics,
    /// Une cible absente ou réduite à un curseur ne produit pas de compteur sélection.
    pub target: Option<TextStatistics>,
}

#[derive(Default)]
pub(super) struct StatisticsCache {
    guard: Option<TargetGuard>,
    counter: Counter,
}

impl Engine {
    pub fn statistics(&self, p: StatisticsParams) -> Result<super::StatisticsResult, CommandError> {
        if !self.open {
            return Err(CommandError::new(ErrorCode::Unavailable, "no_document"));
        }
        // Résoudre d’abord : une cible périmée ne lit ni ne réchauffe le cache.
        let range = match p.target {
            Some(target) => self.resolve_query_target(target)?.range(),
            None => 0..0,
        };
        let guard = self.target_guard();
        let mut cache = self.statistics.borrow_mut();
        if cache.guard != Some(guard) {
            cache.counter = Counter::default();
            cache.guard = Some(guard);
        }
        let (document, target) = cache
            .counter
            .counts(guard.revision, &self.document().view, range);
        Ok(StatisticsResult {
            guard,
            document,
            target,
        })
    }
}
