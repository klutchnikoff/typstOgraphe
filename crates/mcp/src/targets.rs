use crate::args::Target;
use serde_json::Value;
use typstographe_core::engine::{CommandError, ErrorCode};
pub fn portable(value: &Value) -> Result<(), CommandError> {
    let forbidden = match value {
        Value::Object(map) => {
            map.keys()
                .any(|k| matches!(k.as_str(), "guard" | "block_id" | "zone_id" | "render"))
                || map.values().any(|v| portable(v).is_err())
        }
        Value::Array(values) => values.iter().any(|v| portable(v).is_err()),
        _ => false,
    };
    if forbidden {
        Err(CommandError::new(
            ErrorCode::InvalidParams,
            "cli_session_reference",
        ))
    } else {
        Ok(())
    }
}

impl Target {
    pub fn validate(&self) -> Result<(), CommandError> {
        if (self.select.is_some() && self.block.is_some())
            || (self.occurrence.is_some() && self.select.is_none())
            || self.occurrence == Some(0)
            || self.block == Some(0)
        {
            return Err(CommandError::new(
                ErrorCode::InvalidParams,
                "cli_target_options",
            ));
        }
        Ok(())
    }
}
