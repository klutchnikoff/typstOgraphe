//! Une entrée JSON bornée et partagée par les appels, depuis un argument/fichier/stdin.
use crate::args::Parameters;
use serde_json::{Map, Value};
use std::{fs::File, io::Read, path::Path};
use typstographe_core::engine::{CommandError, ErrorCode};

pub const MAX_PARAMETERS_BYTES: usize = typstographe_core::engine::MAX_BATCH_BYTES;

impl Parameters {
    pub fn load(self) -> Result<Map<String, Value>, CommandError> {
        let bytes = if let Some(text) = self.params {
            if text.len() > MAX_PARAMETERS_BYTES {
                return Err(too_large());
            }
            text.into_bytes()
        } else if let Some(path) = self.params_file {
            if path == Path::new("-") {
                read(std::io::stdin().lock())?
            } else {
                read(File::open(path).map_err(io_error)?)?
            }
        } else {
            return Ok(Map::new());
        };
        let value: Value = serde_json::from_slice(&bytes).map_err(CommandError::invalid)?;
        value
            .as_object()
            .cloned()
            .ok_or_else(|| CommandError::new(ErrorCode::InvalidParams, "cli_params_object"))
    }
}
fn read(input: impl Read) -> Result<Vec<u8>, CommandError> {
    read_payload(input, "cli_params_io", "cli_params_too_large")
}
pub(crate) fn read_payload(
    input: impl Read,
    io_reason: &str,
    size_reason: &str,
) -> Result<Vec<u8>, CommandError> {
    let mut bytes = Vec::new();
    input
        .take(MAX_PARAMETERS_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| CommandError::detail(ErrorCode::Io, io_reason, e))?;
    if bytes.len() > MAX_PARAMETERS_BYTES {
        return Err(CommandError::new(ErrorCode::InvalidParams, size_reason));
    }
    Ok(bytes)
}
fn too_large() -> CommandError {
    CommandError::new(ErrorCode::InvalidParams, "cli_params_too_large")
}
fn io_error(error: std::io::Error) -> CommandError {
    CommandError::detail(ErrorCode::Io, "cli_params_io", error)
}
