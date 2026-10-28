//! Functional test transport only. All interpretation and publication belong to core.
use serde::Deserialize;
use serde_json::Value;
use typstographe_core::{
    assets::Assets,
    content::{
        commands::{Command, Style, Switch, TextTarget, TextZone},
        contract::{Error, Guard, State},
        editing::{Direction, EditingSession},
        shared::{SharedDocument, SharedSession},
    },
};

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum EditingStep {
    Select {
        actor: usize,
        target: TextTarget,
    },
    Style {
        actor: usize,
        style: Style,
        value: Switch,
    },
    Insert {
        actor: usize,
        text: String,
    },
    Commands {
        actor: usize,
        commands: Vec<Command>,
    },
    History {
        actor: usize,
        direction: Direction,
    },
}
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    Editing {
        update: Vec<u8>,
        guard: Guard,
        contexts: usize,
        steps: Vec<EditingStep>,
    },
    Commands {
        update: Vec<u8>,
        guard: Guard,
        expected: Guard,
        commands: Vec<Command>,
    },
    Zone {
        update: Vec<u8>,
        guard: Guard,
        zone: TextZone,
    },
    Initialize {
        state: State,
    },
    Inspect {
        update: Vec<u8>,
        expected: Option<State>,
    },
    Publish {
        update: Vec<u8>,
        guard: Guard,
        expected: Guard,
        updates: Vec<Vec<u8>>,
    },
}
pub fn run(args: Value, assets: &Assets) -> Result<Value, String> {
    fn execute(request: Request, assets: &Assets) -> Result<Value, Error> {
        let (shared, state) = match request {
            Request::Editing {
                update,
                guard,
                contexts,
                steps,
            } => {
                if contexts > 8 || steps.len() > 256 {
                    return Err(Error::StateLimitExceeded);
                }
                let mut session = EditingSession::open(guard, &update, assets.clone())?;
                let actors = (0..contexts)
                    .map(|_| session.open_context(&session.guard().clone()))
                    .collect::<Result<Vec<_>, _>>()?;
                let mut events = vec![];
                for step in steps {
                    let guard = session.guard().clone();
                    let actor = match &step {
                        EditingStep::Select { actor, .. }
                        | EditingStep::Style { actor, .. }
                        | EditingStep::Insert { actor, .. }
                        | EditingStep::Commands { actor, .. }
                        | EditingStep::History { actor, .. } => *actor,
                    };
                    let id = *actors.get(actor).ok_or_else(|| Error::InvalidParameters {
                        parameter: "actor".into(),
                    })?;
                    let publication = match step {
                        EditingStep::Select { target, .. } => {
                            session.select(&guard, id, Some(target))?;
                            None
                        }
                        EditingStep::Style { style, value, .. } => {
                            session.typing_style(&guard, id, style, value)?;
                            None
                        }
                        EditingStep::Insert { text, .. } => Some(session.insert(&guard, id, text)?),
                        EditingStep::Commands { commands, .. } => {
                            Some(session.execute(&guard, id, &commands)?)
                        }
                        EditingStep::History { direction, .. } => {
                            Some(session.history(&guard, id, direction)?)
                        }
                    };
                    let contexts = actors
                        .iter()
                        .map(|id| session.context(&session.guard().clone(), *id))
                        .collect::<Result<Vec<_>, _>>()?;
                    events.push(serde_json::json!({"publication":publication,"contexts":contexts}));
                }
                return Ok(
                    serde_json::json!({"state":session.state(),"guard":session.guard(),"update":session.update(),"events":events}),
                );
            }
            Request::Commands {
                update,
                guard,
                expected,
                commands,
            } => {
                let mut session = SharedSession::open(guard, &update, assets.clone())?;
                let prepared = session.prepare_commands(&expected, &commands)?;
                return Ok(serde_json::to_value(session.publish(prepared)?).expect("publication"));
            }
            Request::Zone {
                update,
                guard,
                zone,
            } => {
                let session = SharedSession::open(guard.clone(), &update, assets.clone())?;
                return Ok(
                    serde_json::to_value(session.inspect_zone(&guard, &zone)?).expect("zone")
                );
            }
            Request::Initialize { state } => {
                state.validate(assets)?;
                (SharedDocument::from_state(&state)?, state)
            }
            Request::Inspect { update, expected } => {
                let shared = SharedDocument::from_update(&update)?;
                if let Some(expected) = expected {
                    shared.verify(&expected)?;
                }
                let state = shared.state()?;
                state.validate(assets)?;
                (shared, state)
            }
            Request::Publish {
                update,
                guard,
                expected,
                updates,
            } => {
                let mut session = SharedSession::open(guard, &update, assets.clone())?;
                let prepared = session.prepare(&expected, &updates)?;
                return Ok(serde_json::to_value(session.publish(prepared)?)
                    .expect("publication serialization"));
            }
        };
        Ok(serde_json::json!({"state":state, "update":shared.update()}))
    }
    let request = serde_json::from_value(args).map_err(|e| e.to_string())?;
    execute(request, assets)
        .map_err(|e| serde_json::to_string(&e).expect("contract error serialization"))
}
