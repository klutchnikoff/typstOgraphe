//! Bounded invitation-scoped proposals. No tool can call the human decision path.
use super::{host, policy::Policy, types::DocumentRef};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use typstographe_core::{document::Document, engine::*, storage::StoredDocument};
#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct Submit {
    document: DocumentRef,
    request_id: String,
    #[serde(default)]
    summary: String,
    steps: Vec<Call>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct ProposalId {
    pub proposal: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    Pending,
    Accepted,
    Rejected,
    Stale,
    Cancelled,
}

#[derive(Clone)]
pub struct ProposalReview {
    pub guard: TargetGuard,
    pub id: String,
    pub summary: String,
    pub before: StoredDocument,
    pub after: StoredDocument,
    pub original: Document,
    pub candidate: Document,
}
struct Record {
    id: String,
    request_id: String,
    fingerprint: Vec<u8>,
    status: ProposalStatus,
    guard: Option<TargetGuard>,
}
struct Pending {
    index: usize,
    candidate: PreparedProposal,
    review: ProposalReview,
}
#[derive(Default)]
pub(crate) struct Proposals {
    records: Vec<Record>,
    pending: Option<Pending>,
}

pub(crate) fn error(reason: &str) -> CommandError {
    CommandError::detail(ErrorCode::Unavailable, "mcp_proposal", reason)
}
impl Proposals {
    pub fn refresh(&mut self, engine: &Engine, context: &EditContext) {
        if self
            .pending
            .as_ref()
            .is_some_and(|p| engine.check_proposal(context, &p.candidate).is_err())
        {
            let pending = self.pending.take().expect("pending");
            self.records[pending.index].status = ProposalStatus::Stale;
        }
    }
    pub fn review(&self) -> Option<&ProposalReview> {
        self.pending.as_ref().map(|p| &p.review)
    }
    pub fn status(&self, id: &str) -> Option<ProposalStatus> {
        self.records.iter().find(|r| r.id == id).map(|r| r.status)
    }
    fn response(&self, index: usize) -> Value {
        let r = &self.records[index];
        json!({"proposal":r.id,"request_id":r.request_id,"status":r.status,
            "published":r.status == ProposalStatus::Accepted,"published_guard":r.guard})
    }
    pub fn dispatch(
        &mut self,
        engine: &Engine,
        context: &EditContext,
        document: &str,
        name: &str,
        arguments: Map<String, Value>,
        busy: bool,
    ) -> Result<Value, CommandError> {
        self.refresh(engine, context);
        if name == "proposals.submit" {
            return self.submit(engine, context, document, arguments, busy);
        }
        let p: ProposalId =
            serde_json::from_value(Value::Object(arguments)).map_err(CommandError::invalid)?;
        let index = self
            .records
            .iter()
            .position(|r| r.id == p.proposal)
            .ok_or_else(|| error("Unknown proposal in this invitation"))?;
        if name == "proposals.cancel" && self.records[index].status == ProposalStatus::Pending {
            self.pending = None;
            self.records[index].status = ProposalStatus::Cancelled;
        }
        Ok(self.response(index))
    }
    fn submit(
        &mut self,
        engine: &Engine,
        context: &EditContext,
        document: &str,
        arguments: Map<String, Value>,
        busy: bool,
    ) -> Result<Value, CommandError> {
        let bytes = serde_json::to_vec(&arguments).map_err(CommandError::invalid)?;
        if bytes.len() > MAX_BATCH_BYTES {
            return Err(error("Proposal JSON exceeds the budget"));
        }
        let fingerprint = Sha256::digest(&bytes).to_vec();
        let p: Submit =
            serde_json::from_value(Value::Object(arguments)).map_err(CommandError::invalid)?;
        if let Some(index) = self
            .records
            .iter()
            .position(|r| r.request_id == p.request_id)
        {
            if self.records[index].fingerprint != fingerprint {
                return Err(error("request_id already used for different content"));
            }
            return Ok(self.response(index));
        }
        if busy {
            return Err(error(
                "Finish the active input or dialogue, then submit again explicitly",
            ));
        }
        if p.document.id != document {
            return Err(CommandError::new(
                ErrorCode::Document,
                "mcp_document_foreign",
            ));
        }
        if p.document.guard != engine.target_guard() {
            return Err(CommandError::new(ErrorCode::Document, "stale_revision"));
        }
        if p.request_id.is_empty() || p.request_id.len() > 128 || p.summary.len() > 2048 {
            return Err(error(
                "request_id must contain 1–128 bytes; summary is limited to 2048 bytes",
            ));
        }
        if self.pending.is_some() {
            return Err(error("One proposal is already awaiting a decision"));
        }
        if self.records.len() >= 64 {
            return Err(error(
                "Invitation proposal limit reached; create a new invitation",
            ));
        }
        let policy = Policy::attached_proposals()?;
        let candidate = engine.prepare_resolved_proposal(
            context,
            BatchParams {
                guard: p.document.guard,
                steps: p.steps,
                save: None,
            },
            |_, draft, call| host::resolve_step(draft, call, &policy),
        )?;
        let id = uuid::Uuid::new_v4().to_string();
        let review = ProposalReview {
            guard: engine.target_guard(),
            id: id.clone(),
            summary: p.summary,
            before: engine.stored_document(),
            after: candidate.document().clone(),
            original: engine.document().clone(),
            candidate: candidate.candidate_document().clone(),
        };
        let index = self.records.len();
        self.records.push(Record {
            id,
            request_id: p.request_id,
            fingerprint,
            status: ProposalStatus::Pending,
            guard: None,
        });
        self.pending = Some(Pending {
            index,
            candidate,
            review,
        });
        Ok(self.response(index))
    }
    /// Trusted UI entry point, deliberately absent from dispatch/tools/list.
    pub fn decide(
        &mut self,
        engine: &mut Engine,
        context: &EditContext,
        id: &str,
        accept: bool,
        busy: bool,
    ) -> Result<Option<Outcome>, CommandError> {
        self.refresh(engine, context);
        if accept && busy {
            return Err(error(
                "Finish the active input or dialogue before accepting",
            ));
        }
        let pending = self
            .pending
            .as_ref()
            .filter(|p| p.review.id == id)
            .ok_or_else(|| error("Proposal is no longer pending"))?;
        let outcome = if accept {
            Some(engine.accept_proposal(context, &pending.candidate)?)
        } else {
            None
        };
        let pending = self.pending.take().expect("checked pending");
        let record = &mut self.records[pending.index];
        record.status = if accept {
            ProposalStatus::Accepted
        } else {
            ProposalStatus::Rejected
        };
        record.guard = outcome.as_ref().map(|o| o.guard);
        Ok(outcome)
    }
}

pub(super) fn tools() -> Vec<rmcp::model::Tool> {
    let mut submit = super::catalogue::extra::<Submit>(
        "proposals.submit",
        "Attached session only. Prepare an isolated atomic lot for human review. Returns pending/published=false immediately; no text is published until the user accepts in the application. Reuse request_id only for an identical retry. At most one pending proposal. Steps use the documented editing params/selectors; no files, history, nested batch or final save. Do not poll in a loop; return control to the user.",
    );
    let mut schema = submit.input_schema.as_ref().clone();
    schema["$defs"]["Call"]["properties"]["params"] =
        json!({"type":"object","additionalProperties":true});
    submit.input_schema = std::sync::Arc::new(schema);
    submit.annotations = Some(
        rmcp::model::ToolAnnotations::new()
            .read_only(false)
            .open_world(false),
    );
    let get = super::catalogue::extra::<ProposalId>(
        "proposals.get",
        "Attached session only. Read a proposal status by its invitation-scoped ID, including after the document revision changes. Terminal states: accepted, rejected, stale, cancelled. No document mutation. Do not poll repeatedly while awaiting human review.",
    );
    let mut cancel = super::catalogue::extra::<ProposalId>(
        "proposals.cancel",
        "Attached session only. Withdraw a pending proposal. Terminal states remain unchanged; this never undoes an accepted lot. No tool can accept or approve a proposal.",
    );
    cancel.annotations = Some(
        rmcp::model::ToolAnnotations::new()
            .read_only(false)
            .open_world(false),
    );
    vec![submit, get, cancel]
}
