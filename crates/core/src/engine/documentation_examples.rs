//! Exemples littéraux et références à obtenir du moteur, jamais d’identités inventées.
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ExampleReference {
    ImageBlock,
    SearchOccurrence,
    SpellingIssue,
    Render,
    Recovery,
}
#[derive(Serialize)]
pub(super) struct Binding {
    pub parameter: &'static str,
    pub reference: ExampleReference,
    pub obtain_from: &'static str,
}
#[derive(Serialize)]
pub(super) struct Example {
    pub params: Value,
    pub bindings: Vec<Binding>,
}
pub(super) struct Guide {
    pub details: &'static str,
    pub example: Example,
    pub restrictions: &'static str,
    pub result: &'static str,
}
impl Guide {
    pub fn new(details: &'static str, params: Value) -> Self {
        Self {
            details,
            example: Example {
                params,
                bindings: vec![],
            },
            restrictions: "Availability, guards, structural boundaries and parameter validation are checked by the engine. Adapter permissions apply independently.",
            result: "An outcome with guard, current logical selection, conditional effects and an optional value. Read the actual outcome; a no-op need not create an undo entry.",
        }
    }
    pub fn bind(mut self, parameter: &'static str, reference: ExampleReference) -> Self {
        let obtain_from = match reference {
            ExampleReference::ImageBlock => {
                "Read document.structure; choose an image block and combine its id with the returned guard as {guard, block_id}. The example's target must be replaced as a whole."
            }
            ExampleReference::SearchOccurrence => {
                "Call search.find with the same query/case_sensitive settings; use an exact item from matches, including its expected text and guarded target."
            }
            ExampleReference::SpellingIssue => {
                "Call spelling.issues; use an exact current item from issues, including language, word and guarded target. Requires an available dictionary and an actual issue."
            }
            ExampleReference::Render => {
                "Call render.compile for this revision; use the returned render.reference. A render from another revision is invalid."
            }
            ExampleReference::Recovery => {
                "Read recovery.list from a configured recovery service and copy an existing item's opaque reference. MCP does not expose recovery operations."
            }
        };
        self.example.bindings.push(Binding {
            parameter,
            reference,
            obtain_from,
        });
        self
    }
    pub fn restrict(mut self, text: &'static str) -> Self {
        self.restrictions = text;
        self
    }
    pub fn returns(mut self, text: &'static str) -> Self {
        self.result = text;
        self
    }
}
