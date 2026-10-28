//! Functional test transport only. All interpretation and publication belong to core.
use serde::Deserialize;
use serde_json::Value;
use typstographe_core::{
    assets::Assets,
    content::{
        commands::{Command, TextZone},
        contract::{Error, Guard, State},
        shared::{SharedDocument, SharedSession},
    },
};

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
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
