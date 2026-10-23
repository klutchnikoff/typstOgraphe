//! Références opaques fournies uniquement par le service de récupération configuré.
use super::TargetGuard;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct RecoveryRef(pub(super) u64);
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RestoreRecoveryParams {
    pub guard: TargetGuard,
    pub reference: RecoveryRef,
    #[serde(default)]
    pub discard_changes: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeleteRecoveryParams {
    pub reference: RecoveryRef,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoveryItem {
    pub reference: RecoveryRef,
    pub name: Option<String>,
    pub saved_at: Option<i64>,
    pub error: Option<String>,
}
