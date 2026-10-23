//! Lots bornés : mêmes appels publics, sauvegarde finale distincte des étapes.
use super::{Call, TargetGuard};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// No file is written when save is absent. Current requires an existing session path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BatchSave {
    Current {},
    As {
        path: PathBuf,
        #[serde(default)]
        overwrite: bool,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BatchParams {
    pub guard: TargetGuard,
    #[schemars(length(min = 1, max = super::batch::MAX_BATCH_STEPS))]
    pub steps: Vec<Call>,
    #[serde(default)]
    pub save: Option<BatchSave>,
}
