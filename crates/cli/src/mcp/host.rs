//! Session autonome propriétaire : aucune dépendance au chemin d’un document.
use super::{catalogue, policy, types::*};
use crate::{args::Target, calls};
use serde_json::{Map, Value, json};
use typstographe_core::engine::*;

pub struct Host {
    engine: Engine,
    id: Option<String>,
    policy: policy::Policy,
}
impl Host {
    #[cfg(test)]
    pub fn new(read_only: bool) -> Self {
        Self::with_policy(policy::Policy::new(vec![], vec![], read_only).unwrap())
    }
    pub fn with_policy(policy: policy::Policy) -> Self {
        Self {
            engine: Engine::empty(),
            id: None,
            policy,
        }
    }
    fn reference(&self) -> Option<DocumentRef> {
        self.id.as_ref().map(|id| DocumentRef {
            id: id.clone(),
            guard: self.engine.target_guard(),
        })
    }
    fn check_document(
        &self,
        reference: Option<DocumentRef>,
        required: bool,
    ) -> Result<(), CommandError> {
        let Some(reference) = reference else {
            return if required {
                Err(CommandError::new(
                    ErrorCode::InvalidParams,
                    "mcp_document_required",
                ))
            } else {
                Ok(())
            };
        };
        if self.id.as_ref() != Some(&reference.id) {
            return Err(CommandError::new(
                ErrorCode::Document,
                "mcp_document_foreign",
            ));
        }
        if reference.guard != self.engine.target_guard() {
            return Err(CommandError::new(ErrorCode::Document, "stale_revision"));
        }
        Ok(())
    }
    pub fn dispatch(
        &mut self,
        name: &str,
        arguments: Map<String, Value>,
    ) -> Result<Reply, CommandError> {
        if serde_json::to_vec(&arguments)
            .map_err(CommandError::invalid)?
            .len()
            > MAX_BATCH_BYTES
        {
            return Err(CommandError::new(
                ErrorCode::InvalidParams,
                "batch_size_limit",
            ));
        }
        let (outcome, png) = match name {
            "document_types.describe" => {
                let p: DescribeType = decode(arguments)?;
                (Engine::type_documentation(p.document_type), None)
            }
            "capabilities.describe" => {
                let p: DescribeCapability = decode(arguments)?;
                let mut info = Engine::command_documentation(&p.id)?;
                let spec = typstographe_core::engine::catalogue()
                    .iter()
                    .find(|s| s.id.as_str() == p.id)
                    .expect("described command");
                info["prototype_support"] = self.policy.support(spec);
                info["mcp_input_schema"] = catalogue::tools()
                    .iter()
                    .find(|t| t.name == p.id)
                    .map(|t| json!(t.input_schema))
                    .unwrap_or(Value::Null);
                (info, None)
            }
            "render.page" => {
                let p: RenderPage = decode(arguments)?;
                self.check_document(Some(p.document), true)?;
                let render = calls::compile_render(&self.engine)?;
                let index = p
                    .page
                    .checked_sub(1)
                    .map(|n| n as usize)
                    .filter(|i| *i < render.pages.len())
                    .ok_or_else(|| {
                        CommandError::new(ErrorCode::InvalidParams, "cli_render_page")
                    })?;
                let png = self.engine.render_page_png(render.reference, index)?;
                (
                    json!({"page":p.page,"pages":render.pages.len(),"size_points":render.pages[index]}),
                    Some(png),
                )
            }
            _ => (self.registered(name, arguments)?, None),
        };
        Ok(Reply {
            value: json!({"version":1,"ok":true,"document":self.reference(),"outcome":outcome,"summary":typstographe_i18n::tr!("mcp-completed", id=name),"permissions":self.policy.describe()}),
            png,
        })
    }
    fn registered(
        &mut self,
        name: &str,
        mut params: Map<String, Value>,
    ) -> Result<Value, CommandError> {
        let spec = typstographe_core::engine::catalogue()
            .iter()
            .find(|s| s.id.as_str() == name)
            .ok_or_else(|| CommandError::new(ErrorCode::NotFound, "unknown_command"))?;
        if !policy::exposed(spec) {
            return super::roots::denied();
        }
        let reference = params
            .remove("document")
            .map(serde_json::from_value)
            .transpose()
            .map_err(CommandError::invalid)?;
        self.check_document(
            reference,
            catalogue::needs_document(spec)
                || (matches!(name, "document.new" | "document.open") && self.engine.is_open()),
        )?;
        let selector: Target = params
            .remove("selector")
            .map(serde_json::from_value)
            .transpose()
            .map_err(CommandError::invalid)?
            .unwrap_or_default();
        if params.contains_key("guard") {
            return Err(CommandError::new(
                ErrorCode::InvalidParams,
                "mcp_step_guard",
            ));
        }
        let selection = prepare(&self.engine, spec, &mut params, selector)?;
        if spec.family == Family::Query {
            let value = self
                .engine
                .query(Query::from_call(name, Value::Object(params))?)?;
            return Ok(self.query_response(name, value));
        }
        let mut command = Command::from_call(name, Value::Object(params))?;
        self.policy.prepare(&self.engine, &mut command)?;
        let outcome = if let Command::Batch(p) = command {
            self.engine.run_resolved_batch(p, |_, draft, call| {
                let spec = typstographe_core::engine::catalogue()
                    .iter()
                    .find(|s| s.id == call.id)
                    .expect("preflighted");
                let mut params = call
                    .params
                    .as_object()
                    .cloned()
                    .ok_or_else(|| CommandError::invalid("step params must be an object"))?;
                if params.contains_key("guard") {
                    return Err(CommandError::new(
                        ErrorCode::InvalidParams,
                        "mcp_step_guard",
                    ));
                }
                let selector = params
                    .remove("selector")
                    .map(serde_json::from_value)
                    .transpose()
                    .map_err(CommandError::invalid)?
                    .unwrap_or_default();
                let selection = prepare(draft, spec, &mut params, selector)?;
                let mut command = Command::from_call(call.id.as_str(), Value::Object(params))?;
                self.policy.prepare(draft, &mut command)?;
                Ok(ResolvedBatchStep { command, selection })
            })?
        } else if selection.is_some() && command.batch_step_allowed() {
            // La sélection de confort et l’édition doivent échouer ensemble.
            let mut resolved = Some(ResolvedBatchStep { command, selection });
            let steps = vec![resolved.as_ref().expect("one step").command.to_call()];
            self.engine.run_resolved_batch(
                BatchParams {
                    guard: self.engine.target_guard(),
                    steps,
                    save: None,
                },
                |_, _, _| Ok(resolved.take().expect("one step")),
            )?
        } else {
            self.engine.run(command)?
        };
        if outcome.effects.contains(&Effect::DocumentReplaced) {
            self.id = self
                .engine
                .is_open()
                .then(|| uuid::Uuid::new_v4().to_string());
        }
        Ok(serde_json::to_value(outcome).expect("outcome"))
    }
    fn query_response(&self, name: &str, mut value: Value) -> Value {
        if name == "commands.list" {
            for entry in value["entries"].as_array_mut().expect("entries") {
                let spec = typstographe_core::engine::catalogue()
                    .iter()
                    .find(|s| s.id.as_str() == entry["id"])
                    .expect("registered entry");
                entry["mcp_support"] = self.policy.support(spec);
            }
        }
        if name == "models.list" {
            for entry in value["models"].as_array_mut().expect("models") {
                entry.as_object_mut().expect("model").remove("fixed_page");
            }
        }
        if name == "document_types.list" {
            for kind in value["types"].as_array_mut().expect("types") {
                for model in kind["models"].as_array_mut().expect("models") {
                    model.as_object_mut().expect("model").remove("fixed_page");
                }
            }
        }
        value
    }
}
fn decode<P: serde::de::DeserializeOwned>(
    arguments: Map<String, Value>,
) -> Result<P, CommandError> {
    serde_json::from_value(Value::Object(arguments)).map_err(CommandError::invalid)
}
fn prepare(
    engine: &Engine,
    spec: &CommandSpec,
    params: &mut Map<String, Value>,
    selector: Target,
) -> Result<Option<SelectionTarget>, CommandError> {
    let selection = selector.resolve(engine, spec, params)?;
    if spec.id.as_str() == "render.locate"
        && let Some(selection) = selection
    {
        params["position"]["target"] = json!(TextTarget::from(selection));
    }
    if !spec.params_schema["properties"]["guard"].is_null() {
        params.insert("guard".into(), json!(engine.target_guard()));
    }
    if !spec.params_schema["properties"]["render"].is_null() && !params.contains_key("render") {
        params.insert(
            "render".into(),
            json!(calls::compile_render(engine)?.reference),
        );
    }
    Ok(selection)
}
