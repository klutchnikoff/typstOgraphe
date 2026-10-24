use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use typstographe_core::engine::TargetGuard;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DocumentRef {
    pub id: String,
    pub guard: TargetGuard,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DescribeType {
    pub document_type: typstographe_core::document_types::DocumentType,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DescribeCapability {
    pub id: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RenderPage {
    pub document: DocumentRef,
    #[schemars(range(min = 1))]
    pub page: u32,
}
pub struct Reply {
    pub value: Value,
    pub png: Option<Vec<u8>>,
}
