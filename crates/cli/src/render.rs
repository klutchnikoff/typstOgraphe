//! Une seule page du cache Typst, publiée sans remplacement, sans préférences GUI.
use crate::{Report, calls};
use serde_json::json;
use std::{
    io::Write,
    path::{Path, PathBuf},
};
use typstographe_core::engine::*;
use typstographe_i18n::tr;

pub fn execute(file: PathBuf, page: u32, png: PathBuf) -> Result<Report, CommandError> {
    // Refuser aussi les liens symboliques, y compris pendants, sans suivre la cible.
    match std::fs::symlink_metadata(&png) {
        Ok(_) => return Err(CommandError::new(ErrorCode::Unavailable, "file_exists")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(io_error(error)),
    }
    let mut engine = calls::open(file, false)?;
    let render = engine.query("render.compile", json!({}))?;
    let pages = render["pages"].as_array().unwrap();
    let index = page
        .checked_sub(1)
        .map(|n| n as usize)
        .filter(|index| *index < pages.len())
        .ok_or_else(|| CommandError {
            code: ErrorCode::InvalidParams,
            params: json!({"reason":"cli_render_page","page":page,"pages":pages.len()}),
        })?;
    let bytes = engine.page_png(page)?;
    let parent = png
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(io_error)?;
    temporary.write_all(&bytes).map_err(io_error)?;
    temporary.as_file().sync_all().map_err(io_error)?;
    // persist_noclobber protège aussi la course avec une création concurrente.
    temporary.persist_noclobber(&png).map_err(|error| {
        if error.error.kind() == std::io::ErrorKind::AlreadyExists {
            CommandError::new(ErrorCode::Unavailable, "file_exists")
        } else {
            io_error(error.error)
        }
    })?;
    Ok(Report {
        outcome: json!({"path":png,"page":page,"pages":pages.len(),"size_points":pages[index]}),
        text: format!(
            "{}\n",
            tr!(
                "cli-rendered",
                page = page,
                path = png.display().to_string()
            )
        ),
        exit: 0,
    })
}

fn io_error(error: std::io::Error) -> CommandError {
    CommandError::detail(ErrorCode::Io, "cli_render_io", error)
}
