//! Explicit native classification transitions, with inactive publication data retained losslessly.
use crate::{Profile, Record, Result};
use hayagriva::types::EntryType;

impl Record {
    pub fn reclassified(&self, expected: &crate::Fingerprint, profile: Profile) -> Result<Self> {
        self.guard(expected)?;
        if self.profile() == profile {
            return Ok(self.clone());
        }
        let mut value = self.native_value();
        // Preserve common inherited information before its publication leaves the active graph.
        fn inherited<'a>(
            value: &'a serde_json::Value,
            field: &str,
        ) -> Option<&'a serde_json::Value> {
            value.get(field).or_else(|| {
                value
                    .get("parent")?
                    .as_array()?
                    .iter()
                    .find_map(|p| inherited(p, field))
            })
        }
        for field in ["author", "date", "publisher", "editor", "organization"] {
            if value.get(field).is_none()
                && let Some(data) = inherited(&self.native_value(), field)
            {
                value[field] = data.clone();
            }
        }
        let mut pool = self.retained_parents.clone();
        for parent in self.entry().parents() {
            if !pool.contains(parent) {
                pool.push(parent.clone())
            }
        }
        let mut active = vec![];
        let mut retained = vec![];
        for parent in pool {
            if profile.parent_types().contains(parent.entry_type()) {
                active.push(parent);
            } else {
                retained.push(parent);
            }
        }
        let seed = profile.empty_entry(self.key());
        if active.is_empty() {
            active = seed.parents().to_vec();
        }
        let kind =
            if !active.is_empty() && matches!(profile, Profile::Preprint | Profile::WebResource) {
                EntryType::Article
            } else {
                *seed.entry_type()
            };
        value["type"] = serde_json::json!(kind);
        value["parent"] = serde_json::json!(
            active
                .iter()
                .map(crate::model::canonical_entry)
                .collect::<Vec<_>>()
        );
        let mut record = self.with_entry(value)?;
        record.retained_parents = retained;
        Ok(record)
    }
}
