//! Construction volontaire de fixtures, y compris invalides, uniquement dans les tests.
//! Aucun accès mutable à Session n’est compilé dans l’API de l’application.
use super::*;

pub(crate) fn changed(engine: &mut Engine, selection: Selection) -> Selection {
    let selection = engine.session.changed(selection);
    engine.selection = selection;
    selection
}
pub(crate) fn remember(
    engine: &mut Engine,
    before: NativeEdit,
    selection: Selection,
    typing: bool,
) -> Result<Selection, EditError> {
    let selection = engine.session.remember(before.before, selection, typing)?;
    engine.selection = selection;
    Ok(selection)
}
