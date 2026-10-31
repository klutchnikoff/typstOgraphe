//! Shared version catalogue operations. Hosts own file permissions and final publication.
use super::{Error, Project, SnapshotDetails, SnapshotReason};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum VersionAction {
    Create {
        details: SnapshotDetails,
    },
    Update {
        #[schemars(with = "String")]
        id: Uuid,
        details: SnapshotDetails,
    },
    Delete {
        #[schemars(with = "String")]
        id: Uuid,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VersionChange {
    /// Optimistic guard for index-only edits, which do not advance document revision.
    pub history: String,
    pub action: VersionAction,
}
impl Project {
    pub fn version_content(&self, id: Uuid) -> Result<serde_json::Value, Error> {
        use base64::Engine;
        let version = self.version(id)?;
        let references =
            if let crate::document_types::Metadata::Article(metadata) = &version.metadata {
                crate::bibliography::index(&metadata.references)
                    .map_err(|reason| crate::content::contract::Error::InvalidDocument { reason })?
            } else {
                vec![]
            };
        Ok(
            serde_json::json!({"state":version.state(),"references":references,"resources":version.assets.iter().map(|(path,data)|serde_json::json!({"path":path,"data":base64::engine::general_purpose::STANDARD.encode(data)})).collect::<Vec<_>>()}),
        )
    }
    pub fn history_token(&self) -> String {
        super::ObjectId::of(&serde_json::to_vec(&self.index).expect("version index"))
            .as_str()
            .into()
    }
    pub fn version_catalogue(&self) -> serde_json::Value {
        serde_json::json!({"project_id":self.project_id(),"history":self.history_token(),"limit":super::MAX_SNAPSHOTS,"versions":self.versions()})
    }
    pub fn prepare_version_change(&self, change: &VersionChange) -> Result<Self, Error> {
        if change.history != self.history_token() {
            return Err(crate::errors::StorageError::VersionIndexChanged.into());
        }
        match &change.action {
            VersionAction::Create { details } => {
                self.prepare_version(details.clone(), SnapshotReason::Manual)
            }
            VersionAction::Update { id, details } => {
                self.prepare_version_details(*id, details.clone())
            }
            VersionAction::Delete { id } => self.prepare_remove_version(*id),
        }
    }
}
