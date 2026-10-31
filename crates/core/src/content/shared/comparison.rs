//! Trusted comparison plans still use the existing causal owner and final validation.
use super::{PreparedUpdate, SharedDocument, SharedSession, resources, seed, structure};
use crate::content::{
    archive::Archive,
    commands::ImageResource,
    contract::{Error, Guard},
};
use yrs::{Transact, XmlFragment};
impl SharedSession {
    pub(in crate::content) fn prepare_comparison(
        &self,
        guard: &Guard,
        archive: &Archive,
    ) -> Result<PreparedUpdate, Error> {
        guard.check(self.guard())?;
        archive.validate().map_err(|e| Error::InvalidDocument {
            reason: e.to_string(),
        })?;
        let candidate = SharedDocument::from_update(&self.update())?;
        let mut assets = self.assets.clone();
        for (path, data) in &archive.assets {
            resources::add(
                &mut assets,
                &ImageResource {
                    path: path.clone(),
                    data: data.to_vec(),
                },
            )?;
        }
        // Preserve unchanged shared branches and their anchors. Rebuild only changed runs.
        let old: Vec<_> = self
            .state
            .document
            .content
            .iter()
            .map(|n| serde_json::to_string(n).unwrap())
            .collect();
        let new: Vec<_> = archive
            .document
            .content
            .iter()
            .map(|n| serde_json::to_string(n).unwrap())
            .collect();
        let body = candidate.doc.get_or_insert_xml_fragment("default");
        let ops = crate::content::comparison::sequence_diff(&old, &new);
        for op in ops
            .iter()
            .rev()
            .filter(|op| op.tag() != similar::DiffTag::Equal)
        {
            let a = op.old_range();
            let b = op.new_range();
            let mut tx = candidate.doc.transact_mut();
            body.remove_range(&mut tx, a.start as u32, a.len() as u32);
            seed::children(
                body.clone(),
                &mut tx,
                &archive.document.content[b],
                a.start as u32,
                1,
                &mut 0,
            )?;
        }
        let state = archive.state();
        if state.settings != self.state.settings {
            seed::write_settings(&candidate.doc, &state)?;
        }
        structure::validate(&state)?;
        // Pinned packages belong to the compared state, not the installed catalogue.
        if state.settings != self.state.settings
            || archive.dependencies != crate::packages::for_template(&self.state.settings.template)
        {
            let mut world = crate::rendering::TypstWorld::default();
            world.set_assets(&assets);
            world.set_dependencies(&archive.dependencies);
            world
                .validate_model(
                    &state.validate(&assets)?,
                    &archive.metadata,
                    &archive.template,
                )
                .map_err(|e| Error::InvalidSettings {
                    reason: e.to_string(),
                })?;
        }
        let prepared =
            self.finish_preparation_with_assets(guard, candidate, Some(assets), false)?;
        if prepared.state() != &state {
            return Err(Error::InconsistentState {
                field: "comparison_candidate".into(),
            });
        }
        Ok(prepared)
    }
}
