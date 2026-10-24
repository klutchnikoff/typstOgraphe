//! Des sélecteurs portables résolus sur la structure et la recherche du moteur courant.
use crate::args::Target;
use serde_json::{Map, Value, json};
use typstographe_core::{
    document::structure::{LogicalSelection, targets::Position},
    engine::*,
};

fn refers_to(value: &Value, name: &str) -> bool {
    match value {
        Value::Object(map) => {
            map.get("$ref")
                .and_then(Value::as_str)
                .is_some_and(|s| s.strip_prefix("#/$defs/") == Some(name))
                || map.values().any(|v| refers_to(v, name))
        }
        Value::Array(values) => values.iter().any(|v| refers_to(v, name)),
        _ => false,
    }
}

/// Les gardes et références d’un ancien processus ne sont pas des entrées portables.
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
    pub fn resolve(
        self,
        engine: &Engine,
        spec: &CommandSpec,
        params: &mut Map<String, Value>,
    ) -> Result<Option<SelectionTarget>, CommandError> {
        self.validate()?;
        if self.select.is_none() && self.block.is_none() {
            return Ok(None);
        }
        let properties = &spec.params_schema["properties"];
        if ["target", "issue", "occurrence"]
            .iter()
            .any(|k| params.contains_key(*k))
        {
            return Err(CommandError::new(
                ErrorCode::InvalidParams,
                "cli_params_conflict",
            ));
        }
        let mut block_id = None;
        let mut occurrence = None;
        let selection = if let Some(number) = self.block {
            let structure = engine.query(Query::Structure(Empty {}))?;
            let block = structure["blocks"]
                .as_array()
                .expect("blocks")
                .get(number as usize - 1)
                .ok_or_else(|| CommandError::new(ErrorCode::NotFound, "cli_block_missing"))?;
            let id = serde_json::from_value(block["id"].clone()).expect("block ID");
            block_id = Some(id);
            SelectionTarget {
                guard: engine.target_guard(),
                selection: LogicalSelection {
                    anchor: Position::Before(id),
                    head: Position::After(id),
                },
            }
        } else {
            let results: SearchResults =
                serde_json::from_value(engine.query(Query::Find(SearchParams {
                    query: self.select.expect("select or block"),
                    case_sensitive: true,
                }))?)
                .expect("search results");
            if results.matches.len() > 1 && self.occurrence.is_none() {
                return Err(CommandError::new(
                    ErrorCode::InvalidParams,
                    "cli_selection_ambiguous",
                ));
            }
            let found = results
                .matches
                .get(self.occurrence.unwrap_or(1) as usize - 1)
                .ok_or_else(|| CommandError::new(ErrorCode::NotFound, "cli_selection_missing"))?;
            occurrence = Some(found.clone());
            found.target
        };

        let target_schema = &properties["target"];
        if refers_to(target_schema, "BlockTarget") {
            let block_id = block_id
                .ok_or_else(|| CommandError::new(ErrorCode::InvalidParams, "cli_block_required"))?;
            params.insert(
                "target".into(),
                json!(BlockTarget {
                    guard: engine.target_guard(),
                    block_id
                }),
            );
        } else if refers_to(target_schema, "TextTarget") {
            params.insert("target".into(), json!(TextTarget::from(selection)));
        } else if !properties["issue"].is_null() || refers_to(target_schema, "SuggestionTarget") {
            let issues: SpellingIssues =
                serde_json::from_value(engine.query(Query::SpellingIssues(Empty {}))?)
                    .expect("spelling issues");
            let issue = issues
                .issues
                .into_iter()
                .find(|i| i.target == selection)
                .ok_or_else(|| CommandError::new(ErrorCode::NotFound, "cli_spelling_missing"))?;
            if refers_to(target_schema, "SuggestionTarget") {
                params.insert("target".into(), json!({"kind":"issue","issue":issue}));
            } else {
                params.insert("issue".into(), json!(issue));
            }
        } else if !properties["occurrence"].is_null() {
            let found = occurrence.ok_or_else(|| {
                CommandError::new(ErrorCode::InvalidParams, "cli_selection_required")
            })?;
            params.insert("occurrence".into(), json!(found));
        } else if properties["position"].is_null()
            || params
                .get("position")
                .is_some_and(|p| p["direction"] == "to_source")
        {
            return Err(CommandError::new(
                ErrorCode::InvalidParams,
                "cli_target_unsupported",
            ));
        }
        Ok(Some(selection))
    }
}
