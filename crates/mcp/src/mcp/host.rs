//! Hôte documentaire autonome ou rattaché : identité indépendante du chemin.
mod render;
use super::engine_access::EngineAccess;
use super::{catalogue, policy, types::*};
use crate::{args::Target, calls};
use serde_json::{Map, Value, json};
use typstographe_core::engine::*;

pub struct Host<'a> {
    engine: EngineAccess<'a>,
    id: Option<String>,
    policy: policy::Policy,
    context: Option<EditContext>,
    proposals: Option<&'a mut super::proposals::Proposals>,
    busy: bool,
    saved: Vec<Outcome>,
}
impl<'a> Host<'a> {
    #[cfg(test)]
    pub fn new(read_only: bool) -> Self {
        Self::with_policy(policy::Policy::new(vec![], vec![], read_only).unwrap())
    }
    pub fn with_policy(policy: policy::Policy) -> Self {
        Self {
            engine: EngineAccess::Owned(Box::new(Engine::empty())),
            id: None,
            policy,
            context: None,
            proposals: None,
            busy: false,
            saved: Vec::new(),
        }
    }
    /// Borrow the application's published engine; never open or clone its file.
    pub(crate) fn attached(
        engine: &'a mut Engine,
        id: String,
        context: EditContext,
    ) -> Result<Self, CommandError> {
        engine.editing_context(&context)?;
        Ok(Self {
            engine: EngineAccess::Borrowed(engine),
            id: Some(id),
            policy: policy::Policy::attached_read_only()?,
            context: Some(context),
            proposals: None,
            busy: false,
            saved: Vec::new(),
        })
    }
    pub(crate) fn attached_proposals(
        engine: &'a mut Engine,
        id: String,
        context: EditContext,
        proposals: &'a mut super::proposals::Proposals,
        busy: bool,
    ) -> Result<Self, CommandError> {
        let mut host = Self::attached(engine, id, context)?;
        host.policy = policy::Policy::attached_proposals()?;
        host.proposals = Some(proposals);
        host.busy = busy;
        Ok(host)
    }
    pub(crate) fn with_files(mut self, files: super::file_grant::FileGrant) -> Self {
        self.policy = self.policy.with_files(&self.engine, files);
        self
    }
    pub(crate) fn take_saved(&mut self) -> Vec<Outcome> {
        std::mem::take(&mut self.saved)
    }
    pub(crate) fn begin_export(
        &self,
        mut params: Map<String, Value>,
    ) -> Result<(ExportTask, Reply, std::path::PathBuf), CommandError> {
        if serde_json::to_vec(&params)
            .map_err(CommandError::invalid)?
            .len()
            > MAX_BATCH_BYTES
        {
            return Err(CommandError::new(
                ErrorCode::InvalidParams,
                "batch_size_limit",
            ));
        }
        self.prepare_registered("document.export", &mut params)?;
        let mut command = Command::from_call("document.export", Value::Object(params))?;
        self.policy.prepare(&self.engine, &mut command)?;
        let Command::Export(params) = command else {
            unreachable!()
        };
        let directory = params.directory.clone();
        let task = self.engine.prepare_export(params)?;
        let outcome = Outcome {
            changed: false,
            guard: self.engine.target_guard(),
            selection: self
                .context
                .as_ref()
                .map(|context| {
                    self.engine
                        .editing_context(context)
                        .map(|state| state.selection)
                })
                .transpose()?,
            effects: vec![],
            value: None,
        };
        Ok((
            task,
            self.reply(
                "document.export",
                serde_json::to_value(outcome).expect("outcome"),
                None,
            ),
            directory,
        ))
    }
    fn query(&self, query: Query) -> Result<Value, CommandError> {
        match &self.context {
            Some(context) => self.engine.query_in_context(context, query),
            None => self.engine.query(query),
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
            "proposals.submit" | "proposals.get" | "proposals.cancel" => {
                let store = self.proposals.as_deref_mut().ok_or_else(|| {
                    super::proposals::error("Proposals require an attached invitation")
                })?;
                let context = self.context.as_ref().expect("attached context");
                let id = self.id.as_deref().expect("attached document");
                (
                    store.dispatch(&self.engine, context, id, name, arguments, self.busy)?,
                    None,
                )
            }
            "document_types.describe" => {
                let p: DescribeType = decode(arguments)?;
                (Engine::type_documentation(p.document_type), None)
            }
            "capabilities.describe" => {
                let p: DescribeCapability = decode(arguments)?;
                let info = self.describe_capability(&p.id)?;
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
        Ok(self.reply(name, outcome, png))
    }
    fn reply(&self, name: &str, outcome: Value, png: Option<Vec<u8>>) -> Reply {
        Reply {
            value: json!({"version":1,"ok":true,"document":self.reference(),"outcome":outcome,"summary":typstographe_i18n::tr!("mcp-completed", id=name),"permissions":self.policy.describe()}),
            png,
        }
    }
    fn describe_capability(&self, id: &str) -> Result<Value, CommandError> {
        super::documentation::describe(id, &self.query(Query::Commands(Empty {}))?, &self.policy)
    }
    pub fn resource(&self, uri: &str) -> Result<Reply, CommandError> {
        use typstographe_core::document_types::DocumentType;
        let outcome = match uri {
            "typstographe://content-capabilities" => {
                json!({"entries":typstographe_core::content::catalogue::ALL.iter().map(|capability| typstographe_core::content::catalogue::describe(capability.id).expect("catalogue")).collect::<Vec<_>>()})
            }
            "typstographe://guide" => {
                json!({"instructions":super::instructions(self.policy.is_attached()),"session":if self.policy.is_attached() {"attached"} else {"autonomous"},
                "resources":super::resources::list(),"resource_consistency":"Each read is a current snapshot serialized with tool calls; no subscriptions. Read again after edits. The outer document reference guards subsequent calls."})
            }
            "typstographe://types" => {
                json!({"types":DocumentType::ALL.map(Engine::type_documentation)})
            }
            "typstographe://commands" => {
                let context = self.query(Query::Commands(Empty {}))?;
                let mut entries: Vec<_> = typstographe_core::engine::catalogue()
                    .iter()
                    .map(|s| s.id.as_str())
                    .chain([
                        "document_types.describe",
                        "capabilities.describe",
                        "render.page",
                    ])
                    .map(|id| super::documentation::describe(id, &context, &self.policy))
                    .collect::<Result<_, _>>()?;
                if self.proposals.is_some() {
                    for id in ["proposals.submit", "proposals.get", "proposals.cancel"] {
                        entries.push(super::documentation::describe(id, &context, &self.policy)?);
                    }
                }
                json!({"entries":entries,"tools":catalogue::for_session(self.proposals.is_some()),
                    "extra_tools":"Discovery/rasterization helpers and, for attached sessions, proposals.submit/get/cancel. No approval tool. Their schemas and descriptions are in tools."})
            }
            "typstographe://document" => {
                let mut snapshot = json!({"state":self.query(Query::Inspect(Empty {}))?,
                    "text":null,"structure":null,"metadata":null,"source":null});
                if self.engine.is_open() {
                    for (key, query) in [
                        (
                            "text",
                            Query::Text(TextQueryParams {
                                target: TextTarget::All {},
                            }),
                        ),
                        ("structure", Query::Structure(Empty {})),
                        ("metadata", Query::Metadata(Empty {})),
                        ("source", Query::Source(Empty {})),
                    ] {
                        snapshot[key] = self.query(query)?;
                    }
                }
                snapshot
            }
            "typstographe://models" => {
                let types: Vec<_> = DocumentType::ALL
                    .into_iter()
                    .map(|kind| {
                        let doc = Engine::type_documentation(kind);
                        json!({"type":kind,"models":doc["models"]})
                    })
                    .collect();
                let active = self.engine.is_open().then(|| {
                    self.query_response(
                        "models.list",
                        self.query(Query::Models(Empty {})).expect("models query"),
                    )
                });
                json!({"types":types,"active_document":active})
            }
            _ => {
                return Err(CommandError::new(
                    ErrorCode::NotFound,
                    "mcp_resource_unknown",
                ));
            }
        };
        Ok(Reply {
            value: json!({"version":1,"document":self.reference(),"permissions":self.policy.describe(),"outcome":outcome}),
            png: None,
        })
    }
    fn registered(
        &mut self,
        name: &str,
        mut params: Map<String, Value>,
    ) -> Result<Value, CommandError> {
        let (spec, selection) = self.prepare_registered(name, &mut params)?;
        if spec.family == Family::Query {
            let value = self.query(Query::from_call(name, Value::Object(params))?)?;
            return Ok(self.query_response(name, value));
        }
        let mut command = Command::from_call(name, Value::Object(params))?;
        self.policy.prepare(&self.engine, &mut command)?;
        if let Some(context) = &self.context
            && let Command::SelectText(params) = command
        {
            if self
                .proposals
                .as_ref()
                .is_some_and(|s| s.review().is_some())
            {
                return Err(CommandError::new(
                    ErrorCode::Unavailable,
                    "mcp_attached_busy",
                ));
            }
            let state = self.engine.navigate_context(context, params)?;
            return Ok(json!(Outcome {
                changed: false,
                guard: self.engine.target_guard(),
                selection: Some(state.selection),
                effects: vec![],
                value: None
            }));
        }
        let mut outcome = if let Command::Batch(p) = command {
            self.engine
                .run_resolved_batch(p, |_, draft, call| resolve_step(draft, call, &self.policy))?
        } else if selection.is_some() && command.batch_step_allowed() {
            // La sélection de confort et l’édition doivent échouer ensemble.
            let mut resolved = Some(ResolvedBatchStep { command, selection });
            let steps = vec![resolved.as_ref().expect("one step").command.to_call()];
            let guard = self.engine.target_guard();
            self.engine.run_resolved_batch(
                BatchParams {
                    guard,
                    steps,
                    save: None,
                },
                |_, _, _| Ok(resolved.take().expect("one step")),
            )?
        } else {
            self.engine.run(command)?
        };
        if self.policy.is_attached() && outcome.effects.contains(&Effect::Saved) {
            self.saved.push(outcome.clone());
        }
        if let Some(context) = &self.context {
            // File operations leave both participants in place. Their MCP response
            // must not implicitly transfer the human's selection to the agent.
            outcome.selection = Some(self.engine.editing_context(context)?.selection);
        }
        if outcome.effects.contains(&Effect::DocumentReplaced) {
            self.id = self
                .engine
                .is_open()
                .then(|| uuid::Uuid::new_v4().to_string());
        }
        Ok(serde_json::to_value(outcome).expect("outcome"))
    }
    fn prepare_registered(
        &self,
        name: &str,
        params: &mut Map<String, Value>,
    ) -> Result<(&'static CommandSpec, Option<SelectionTarget>), CommandError> {
        let spec = typstographe_core::engine::catalogue()
            .iter()
            .find(|s| s.id.as_str() == name)
            .ok_or_else(|| CommandError::new(ErrorCode::NotFound, "unknown_command"))?;
        if !policy::exposed(spec) {
            return super::roots::denied();
        }
        if self.policy.is_attached() && spec.family != Family::Query {
            if !self.policy.allows_attached_command(name) {
                return Err(CommandError::new(
                    ErrorCode::Unavailable,
                    "mcp_attached_read_only",
                ));
            }
            if self.busy {
                return Err(CommandError::new(
                    ErrorCode::Unavailable,
                    "mcp_attached_busy",
                ));
            }
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
        let selector = take_selector(spec, params)?;
        if params.contains_key("guard") {
            return Err(CommandError::new(
                ErrorCode::InvalidParams,
                "mcp_step_guard",
            ));
        }
        let selection = prepare(&self.engine, spec, params, selector)?;
        Ok((spec, selection))
    }
    fn query_response(&self, name: &str, mut value: Value) -> Value {
        if name == "commands.list" {
            for entry in value["entries"].as_array_mut().expect("entries") {
                let spec = typstographe_core::engine::catalogue()
                    .iter()
                    .find(|s| s.id.as_str() == entry["id"])
                    .expect("registered entry");
                entry["mcp_support"] = self.policy.support(spec);
                entry["mcp_input_schema"] = catalogue::tools()
                    .iter()
                    .find(|t| t.name == spec.id.as_str())
                    .map(|t| json!(t.input_schema))
                    .unwrap_or(Value::Null);
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
pub(super) fn resolve_step(
    draft: &Engine,
    call: &Call,
    policy: &policy::Policy,
) -> Result<ResolvedBatchStep, CommandError> {
    let spec = typstographe_core::engine::catalogue()
        .iter()
        .find(|s| s.id == call.id)
        .expect("preflighted");
    let mut params = call
        .params
        .as_object()
        .cloned()
        .ok_or_else(|| CommandError::invalid("step params must be an object"))?;
    if params.contains_key("guard") || params.contains_key("document") {
        return Err(CommandError::new(
            ErrorCode::InvalidParams,
            "mcp_step_guard",
        ));
    }
    let selector = take_selector(spec, &mut params)?;
    let selection = prepare(draft, spec, &mut params, selector)?;
    let mut command = Command::from_call(call.id.as_str(), Value::Object(params))?;
    policy.prepare(draft, &mut command)?;
    Ok(ResolvedBatchStep { command, selection })
}
fn decode<P: serde::de::DeserializeOwned>(
    arguments: Map<String, Value>,
) -> Result<P, CommandError> {
    serde_json::from_value(Value::Object(arguments)).map_err(CommandError::invalid)
}
fn take_selector(
    spec: &CommandSpec,
    params: &mut Map<String, Value>,
) -> Result<Target, CommandError> {
    let Some(value) = params.remove("selector") else {
        return Ok(Target::default());
    };
    if !catalogue::supports_selector(spec) {
        return Err(CommandError::new(
            ErrorCode::InvalidParams,
            "cli_target_unsupported",
        ));
    }
    serde_json::from_value(value).map_err(CommandError::invalid)
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
