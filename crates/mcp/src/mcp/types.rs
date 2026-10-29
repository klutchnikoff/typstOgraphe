use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use typstographe_core::content::contract::Guard;
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DocumentRef {
    pub id: String,
    pub guard: Guard,
}
pub struct Reply {
    pub value: serde_json::Value,
    pub png: Option<Vec<u8>>,
}
