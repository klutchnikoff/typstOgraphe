//! Les chemins restent dans le service : aucune commande n’accepte un chemin de récupération arbitraire.
use super::*;
use serde_json::json;
use std::sync::atomic::AtomicU64;
static NEXT_RECOVERY: AtomicU64 = AtomicU64::new(1);

#[derive(Default)]
pub(super) struct RecoveryState {
    configured: bool,
    entries: Vec<(RecoveryRef, crate::recovery::Entry)>,
    adopted: Option<crate::recovery::Entry>,
}
impl Engine {
    pub fn configure_recovery(&mut self, entries: Vec<crate::recovery::Entry>) {
        self.recovery.configured = true;
        self.recovery.entries = entries
            .into_iter()
            .map(|e| (RecoveryRef(crate::identity::next(&NEXT_RECOVERY)), e))
            .collect();
    }
    /// L’adaptateur transfère le verrou au worker existant après une restauration réussie.
    /// Jusqu’à ce transfert, le moteur conserve la copie et son verrou.
    pub fn take_recovered_entry(&mut self) -> Option<crate::recovery::Entry> {
        self.recovery.adopted.take()
    }
    pub fn recovery_items(&self) -> Vec<RecoveryItem> {
        self.recovery
            .entries
            .iter()
            .map(|(reference, entry)| RecoveryItem {
                reference: *reference,
                name: entry
                    .info
                    .as_ref()
                    .ok()
                    .and_then(|i| i.original_name.clone()),
                saved_at: entry.info.as_ref().ok().map(|i| i.saved_at),
                error: entry.info.as_ref().err().cloned(),
            })
            .collect()
    }
    fn recovery_index(&self, reference: RecoveryRef) -> Result<usize, CommandError> {
        if !self.recovery.configured {
            return Err(CommandError::new(
                ErrorCode::Unavailable,
                "recovery_unconfigured",
            ));
        }
        self.recovery
            .entries
            .iter()
            .position(|(r, _)| *r == reference)
            .ok_or_else(|| CommandError::new(ErrorCode::NotFound, "recovery_not_found"))
    }
    pub(super) fn recovery_query(&self) -> Value {
        json!({"configured":self.recovery.configured,"entries":self.recovery_items()})
    }
    pub(super) fn check_recovery(&self, command: &Command) -> Result<(), CommandError> {
        match command {
            Command::RestoreRecovery(p) => {
                self.check_replacement(p.guard, p.discard_changes)?;
                self.recovery_index(p.reference)?;
            }
            Command::DeleteRecovery(p) => {
                self.recovery_index(p.reference)?;
            }
            _ => {}
        }
        Ok(())
    }
    pub(super) fn restore_recovery(
        &mut self,
        p: RestoreRecoveryParams,
    ) -> Result<Value, CommandError> {
        let index = self.recovery_index(p.reference)?;
        let entry = &self.recovery.entries[index].1;
        let stored = entry
            .load()
            .map_err(|e| CommandError::detail(ErrorCode::Io, "recovery_load", e))?;
        let name = entry
            .info
            .as_ref()
            .ok()
            .and_then(|i| i.original_name.clone());
        self.install_stored(stored, None, true)?;
        let (_, entry) = self.recovery.entries.remove(index);
        self.recovery.adopted = Some(entry);
        Ok(json!({"recovered":true,"original_name":name}))
    }
    pub(super) fn delete_recovery(
        &mut self,
        p: DeleteRecoveryParams,
    ) -> Result<Value, CommandError> {
        let index = self.recovery_index(p.reference)?;
        self.recovery.entries[index]
            .1
            .delete()
            .map_err(|e| CommandError::detail(ErrorCode::Io, "recovery_delete", e))?;
        self.recovery.entries.remove(index);
        Ok(json!({"deleted":p.reference}))
    }
}
