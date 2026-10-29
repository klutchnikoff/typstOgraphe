//! The advertised MCP schema is also checked at execution, including native invitations.
use serde_json::Value;
use std::{collections::BTreeMap, sync::OnceLock};
use typstographe_core::{
    content::api,
    protocol::{CommandError, ErrorCode},
};
pub fn check(attached: bool, id: &str, params: &Value) -> Result<(), CommandError> {
    static DIRECT: OnceLock<BTreeMap<String, jsonschema::Validator>> = OnceLock::new();
    static ATTACHED: OnceLock<BTreeMap<String, jsonschema::Validator>> = OnceLock::new();
    let validators = if attached { &ATTACHED } else { &DIRECT }.get_or_init(|| {
        super::catalogue::tools(attached)
            .into_iter()
            .map(|tool| {
                let schema = serde_json::to_value(&tool.input_schema).expect("tool schema");
                (
                    tool.name.into_owned(),
                    jsonschema::validator_for(&schema).expect("MCP schema"),
                )
            })
            .collect()
    });
    validators
        .get(id)
        .ok_or_else(|| CommandError::new(ErrorCode::NotFound, "unknown_command"))?
        .validate(params)
        .map_err(|_| api::content_error(api::invalid(id)))
}
