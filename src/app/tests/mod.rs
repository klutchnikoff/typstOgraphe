#![allow(
    clippy::too_many_lines,
    reason = "Les scénarios d’interaction conservent leurs assertions ; le seuil concerne la production."
)]

use super::*;
use support::*;

mod annotations;
mod annotations_engine;
mod commands;
mod edit_validation;
mod engine;
mod files;
mod images;
mod ime;
mod information;
mod preview;
mod profile;
mod recovery;
mod search;
mod spelling;
mod support;
mod text_input;
mod toolbar;
