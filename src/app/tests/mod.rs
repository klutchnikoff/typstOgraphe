#![allow(
    clippy::too_many_lines,
    reason = "Les scénarios d’interaction conservent leurs assertions ; le seuil concerne la production."
)]

use super::*;
use crate::engine::test_support::with_session as fixture;
use crate::{document::Mark, session::Session};
use support::*;

mod annotations;
mod annotations_engine;
mod batch_engine;
mod blocks_engine;
mod commands;
mod edit_validation;
mod engine;
mod files;
mod files_engine;
mod images;
mod images_engine;
mod ime;
mod information;
mod information_engine;
mod models_engine;
mod preview;
mod profile;
mod recovery;
mod search;
mod spelling;
mod spelling_engine;
mod support;
mod text_input;
mod toolbar;
