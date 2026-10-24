//! Budget commun des ressources conservées par le candidat, sans purge en mémoire.
use super::*;
use crate::assets::MAX_DOCUMENT_BYTES;

pub(super) fn check_budget(candidate: &Snapshot) -> Result<(), CommandError> {
    let dependencies = candidate
        .dependencies
        .values()
        .map(|data| data.len())
        .sum::<usize>();
    if candidate
        .document
        .assets
        .values()
        .map(|data| data.len())
        .sum::<usize>()
        + dependencies
        <= MAX_DOCUMENT_BYTES as usize
    {
        return Ok(());
    }
    let mut document = candidate.document.clone();
    document.sync_source();
    let retained = crate::rendering::retained_assets(
        document.source(),
        &candidate.metadata,
        &candidate.template,
        &document.assets,
        &candidate.dependencies,
    )
    .map_err(|error| CommandError::with_cause(ErrorCode::Document, "asset_validation", error))?;
    if retained.values().map(|data| data.len()).sum::<usize>() + dependencies
        > MAX_DOCUMENT_BYTES as usize
    {
        return Err(CommandError::new(ErrorCode::Document, "document_too_large"));
    }
    Ok(())
}
