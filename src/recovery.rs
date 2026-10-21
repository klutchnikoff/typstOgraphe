//! Copies de récupération autonomes, hors du fichier utilisateur et du rendu.
//! Le worker possède le verrou de sa session ; les autres instances l’ignorent.
use crate::storage::{self, StoredDocument};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub const INTERVAL: Duration = Duration::from_secs(5);
const SNAPSHOT: &str = "document.recovery";
const INFO: &str = "recovery.json";
const INFO_LIMIT: u64 = storage::EXTRA_ENTRY_LIMIT;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Info {
    version: u32,
    pub original_name: Option<String>,
    pub saved_at: i64,
}
impl Info {
    fn new(original_name: Option<String>) -> Self {
        Self {
            version: 1,
            original_name,
            saved_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
        }
    }
    pub fn date(&self) -> String {
        time::OffsetDateTime::from_unix_timestamp(self.saved_at)
            .ok()
            // UTC explicite : pas de dépendance aux réglages de fuseau du worker.
            .and_then(|date| {
                date.format(&time::format_description::well_known::Rfc3339)
                    .ok()
            })
            .unwrap_or_default()
    }
}

struct Slot {
    directory: PathBuf,
    lock: Option<File>,
}
impl Slot {
    fn create(root: &Path) -> Result<Self, String> {
        let directory = tempfile::Builder::new()
            .prefix("session-")
            .tempdir_in(root)
            .map_err(|e| e.to_string())?;
        let lock = File::create_new(directory.path().join("lock")).map_err(|e| e.to_string())?;
        lock.try_lock().map_err(|e| e.to_string())?;
        Ok(Self {
            directory: directory.keep(),
            lock: Some(lock),
        })
    }
    fn claim(directory: PathBuf) -> Result<Option<Self>, String> {
        let lock = File::options()
            .read(true)
            .write(true)
            .open(directory.join("lock"))
            .map_err(|e| e.to_string())?;
        match lock.try_lock() {
            Ok(()) => Ok(Some(Self {
                directory,
                lock: Some(lock),
            })),
            Err(std::fs::TryLockError::WouldBlock) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }
    fn path(&self) -> PathBuf {
        self.directory.join(SNAPSHOT)
    }
    fn clear(&self) -> Result<(), String> {
        match fs::remove_file(self.path()) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }
}
impl Drop for Slot {
    fn drop(&mut self) {
        // Une copie non résolue n’est jamais effacée implicitement. Seuls les
        // répertoires devenus vides sont nettoyés ; pas de suppression récursive.
        if !self.path().exists() {
            self.lock.take();
            let _ = fs::remove_file(self.directory.join("lock"));
            let _ = fs::remove_dir(&self.directory);
        }
    }
}

pub struct Entry {
    slot: Slot,
    pub info: Result<Info, String>,
}
fn open_snapshot(path: &Path) -> Result<File, String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > crate::assets::MAX_DOCUMENT_BYTES + 512 * 1024 {
        return Err(crate::tr!("storage-file-too-large"));
    }
    File::open(path).map_err(|e| e.to_string())
}
fn decode_info(text: &str) -> Result<Info, String> {
    let info: Info = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if info.version != 1
        || time::OffsetDateTime::from_unix_timestamp(info.saved_at).is_err()
        || info
            .original_name
            .as_ref()
            .is_some_and(|name| name.len() > 1024 || name.contains(['/', '\\', '\0']))
    {
        return Err(crate::tr!("storage-unsupported"));
    }
    Ok(info)
}
impl Entry {
    fn from_slot(slot: Slot) -> Self {
        let info = (|| {
            let mut zip =
                zip::ZipArchive::new(open_snapshot(&slot.path())?).map_err(|e| e.to_string())?;
            let file = zip.by_name(INFO).map_err(|e| e.to_string())?;
            if file.size() > INFO_LIMIT {
                return Err(crate::tr!("storage-invalid-size"));
            }
            let mut text = String::new();
            file.take(INFO_LIMIT + 1)
                .read_to_string(&mut text)
                .map_err(|e| e.to_string())?;
            if text.len() as u64 > INFO_LIMIT {
                return Err(crate::tr!("storage-invalid-size"));
            }
            decode_info(&text)
        })();
        Self { slot, info }
    }
    pub fn load(&self) -> Result<StoredDocument, String> {
        let (document, info) =
            storage::decode_archive(open_snapshot(&self.slot.path())?, Some(INFO))?;
        decode_info(&info.ok_or_else(|| crate::tr!("storage-unsupported"))?)?;
        Ok(document)
    }
    pub fn name(&self) -> String {
        self.info
            .as_ref()
            .ok()
            .and_then(|info| info.original_name.clone())
            .unwrap_or_else(|| crate::tr!("untitled-letter"))
    }
    pub fn delete(&self) -> Result<(), String> {
        self.slot.clear()
    }
}

fn scan(root: &Path) -> Result<Vec<Entry>, String> {
    let mut entries = Vec::new();
    for directory in fs::read_dir(root).map_err(|e| e.to_string())? {
        let directory = directory.map_err(|e| e.to_string())?;
        if !directory.file_type().map_err(|e| e.to_string())?.is_dir()
            || !directory
                .file_name()
                .to_string_lossy()
                .starts_with("session-")
            || !directory.path().join(SNAPSHOT).exists()
        {
            continue;
        }
        if let Some(slot) = Slot::claim(directory.path())? {
            entries.push(Entry::from_slot(slot));
        }
    }
    entries.sort_by_key(|entry| {
        std::cmp::Reverse(entry.info.as_ref().map_or(0, |info| info.saved_at))
    });
    Ok(entries)
}

enum Task {
    Write {
        epoch: u64,
        document: Box<StoredDocument>,
        info: Info,
    },
    Clear {
        epoch: u64,
    },
    Adopt {
        epoch: u64,
        slot: Slot,
    },
    #[cfg(test)]
    Barrier(Sender<()>),
}
struct Completed {
    epoch: u64,
    result: Result<(), String>,
}
pub struct Recovery {
    sender: Option<Sender<Task>>,
    receiver: Receiver<Completed>,
    worker: Option<JoinHandle<()>>,
    last_check: Instant,
    submitted: Option<u64>,
    busy: bool,
    epoch: u64,
}
impl Recovery {
    pub fn start(root: &Path, ctx: eframe::egui::Context) -> Result<(Self, Vec<Entry>), String> {
        fs::create_dir_all(root).map_err(|e| e.to_string())?;
        let entries = scan(root)?;
        let mut slot = Slot::create(root)?;
        let (sender, tasks) = mpsc::channel();
        let (results, receiver) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("typstographe-recovery".into())
            .spawn(move || {
                for task in tasks {
                    let (epoch, result) = match task {
                        Task::Write {
                            epoch,
                            document,
                            info,
                        } => {
                            let result = serde_json::to_string(&info)
                                .map_err(|e| e.to_string())
                                .and_then(|info| {
                                    storage::save_archive(
                                        &slot.path(),
                                        &document,
                                        Some((INFO, &info)),
                                    )
                                });
                            (epoch, result)
                        }
                        Task::Clear { epoch } => (epoch, slot.clear()),
                        Task::Adopt {
                            epoch,
                            slot: recovered,
                        } => {
                            slot = recovered;
                            (epoch, Ok(()))
                        }
                        #[cfg(test)]
                        Task::Barrier(sender) => {
                            let _ = sender.send(());
                            continue;
                        }
                    };
                    let _ = results.send(Completed { epoch, result });
                    ctx.request_repaint();
                }
            })
            .map_err(|e| e.to_string())?;
        Ok((
            Self {
                sender: Some(sender),
                receiver,
                worker: Some(worker),
                last_check: Instant::now(),
                submitted: None,
                busy: false,
                epoch: 0,
            },
            entries,
        ))
    }
    fn send(&self, task: Task) -> Result<(), String> {
        self.sender
            .as_ref()
            .unwrap()
            .send(task)
            .map_err(|_| crate::tr!("recovery-worker-stopped"))
    }
    pub fn checkpoint(
        &mut self,
        revision: u64,
        dirty: bool,
        snapshot: impl FnOnce() -> (StoredDocument, Option<String>),
    ) -> Result<(), String> {
        if self.last_check.elapsed() < INTERVAL {
            return Ok(());
        }
        self.last_check = Instant::now();
        if !dirty {
            if self.submitted.is_some() {
                self.clear()?;
            }
        } else if !self.busy && self.submitted != Some(revision) {
            let (document, name) = snapshot();
            self.send(Task::Write {
                epoch: self.epoch,
                document: Box::new(document),
                info: Info::new(name),
            })?;
            self.submitted = Some(revision);
            self.busy = true;
        }
        Ok(())
    }
    pub fn poll(&mut self) -> Vec<Result<(), String>> {
        let mut results = vec![];
        loop {
            let completed = match self.receiver.try_recv() {
                Ok(completed) => completed,
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.busy = false;
                    self.submitted = None;
                    results.push(Err(crate::tr!("recovery-worker-stopped")));
                    break;
                }
            };
            if completed.epoch == self.epoch {
                self.busy = false;
                if completed.result.is_err() {
                    self.submitted = None; // Une prochaine échéance pourra réessayer.
                }
                results.push(completed.result);
            } else if let Err(error) = completed.result {
                // Les erreurs de nettoyage restent utiles après un changement de document.
                results.push(Err(error));
            }
        }
        results
    }
    pub fn clear(&mut self) -> Result<(), String> {
        self.epoch += 1;
        self.submitted = None;
        self.busy = false;
        self.last_check = Instant::now();
        self.send(Task::Clear { epoch: self.epoch })
    }
    pub fn adopt(&mut self, entry: Entry) -> Result<(), String> {
        self.epoch += 1;
        self.submitted = None;
        self.busy = false;
        self.last_check = Instant::now();
        self.send(Task::Adopt {
            epoch: self.epoch,
            slot: entry.slot,
        })
    }
    #[cfg(test)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    pub(crate) fn make_due(&mut self) {
        self.last_check = Instant::now() - INTERVAL;
    }
    #[cfg(test)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    pub(crate) fn wait(&mut self) {
        let (sender, receiver) = mpsc::channel();
        self.send(Task::Barrier(sender)).unwrap();
        receiver.recv_timeout(Duration::from_secs(30)).unwrap();
    }
}
impl Drop for Recovery {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::document_types::DocumentType;

    fn checkpoint(recovery: &mut Recovery, revision: u64, document: &StoredDocument) {
        recovery.make_due();
        recovery
            .checkpoint(revision, true, || (document.clone(), None))
            .unwrap();
        recovery.wait();
        assert!(recovery.poll().into_iter().all(|result| result.is_ok()));
    }

    #[test]
    fn checkpoints_are_periodic_and_keep_a_complete_untitled_document_without_rendering() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = eframe::egui::Context::default();
        let (mut recovery, entries) = Recovery::start(dir.path(), ctx.clone()).unwrap();
        assert!(entries.is_empty());
        // Ni démarrage, ni document propre : aucune copie et aucune sérialisation.
        recovery
            .checkpoint(1, true, || panic!("échéance non atteinte"))
            .unwrap();
        recovery.make_due();
        recovery
            .checkpoint(1, false, || panic!("document propre"))
            .unwrap();
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1); // Répertoire verrouillé, vide.
        let mut document = StoredDocument::blank(DocumentType::CompteRendu);
        document.source =
            "Un *été* à São Paulo.\n\n#figure(image(\"images/parc.svg\"), caption: [Légende.])"
                .into();
        document.assets.insert(
            "images/parc.svg".into(),
            include_bytes!("../tests/fixtures/parc.svg")
                .as_slice()
                .into(),
        );
        // Le modèle est syntaxiquement valide, mais ne compose pas : récupération indépendante.
        document.template = "#let exemple = 1".into();
        checkpoint(&mut recovery, 2, &document);
        recovery.make_due();
        recovery
            .checkpoint(2, true, || panic!("révision déjà copiée"))
            .unwrap();
        let (other, entries) = Recovery::start(dir.path(), ctx.clone()).unwrap();
        assert!(
            entries.is_empty(),
            "une instance vivante ne doit pas être récupérée"
        );
        drop(other);
        drop(recovery); // Comme une fin sans décision, conserve le dernier état durable.
        let (next, entries) = Recovery::start(dir.path(), ctx).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].info.as_ref().unwrap().original_name.is_none());
        assert!(!entries[0].info.as_ref().unwrap().date().is_empty());
        assert_eq!(entries[0].load().unwrap(), document);
        assert!(storage::read(&entries[0].slot.path()).is_err()); // Entrée privée refusée comme document utilisateur.
        drop(entries); // « Plus tard » conserve la copie.
        assert_eq!(scan(dir.path()).unwrap().len(), 1);
        drop(next);
    }

    #[test]
    fn failed_checkpoint_preserves_the_previous_copy_and_can_be_retried() {
        let dir = tempfile::tempdir().unwrap();
        let (mut recovery, _) = Recovery::start(dir.path(), Default::default()).unwrap();
        let document = StoredDocument::demo(DocumentType::Courrier);
        checkpoint(&mut recovery, 1, &document);
        let mut invalid = document.clone();
        invalid.source = "#fonction-inconnue[Erreur]".into();
        recovery.make_due();
        recovery.checkpoint(2, true, || (invalid, None)).unwrap();
        recovery.wait();
        assert!(recovery.poll().iter().any(|result| result.is_err()));
        assert!(recovery.submitted.is_none());
        drop(recovery);
        let mut entries = scan(dir.path()).unwrap();
        let entry = entries.pop().unwrap();
        assert_eq!(entry.load().unwrap(), document);
        let (mut next, _) = Recovery::start(dir.path(), Default::default()).unwrap();
        next.adopt(entry).unwrap();
        let mut edited = document;
        edited.source = "Nouvelle version.".into();
        checkpoint(&mut next, 3, &edited);
        drop(next);
        let entries = scan(dir.path()).unwrap();
        assert_eq!(entries.len(), 1, "pas de doublon lors de la reprise");
        assert_eq!(entries[0].load().unwrap(), edited);
    }

    #[test]
    fn clear_is_ordered_after_pending_writes_and_corrupt_copies_remain_explicitly_deletable() {
        let dir = tempfile::tempdir().unwrap();
        let (mut recovery, _) = Recovery::start(dir.path(), Default::default()).unwrap();
        recovery.make_due();
        recovery
            .checkpoint(1, true, || {
                (StoredDocument::demo(DocumentType::Courrier), None)
            })
            .unwrap();
        recovery.clear().unwrap(); // Peut être appelé pendant la compression précédente.
        drop(recovery); // Attend la file : aucune ancienne écriture ne ressuscite la copie.
        assert!(scan(dir.path()).unwrap().is_empty());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
        let slot = Slot::create(dir.path()).unwrap();
        fs::write(slot.path(), b"archive interrompue").unwrap();
        drop(slot);
        let entries = scan(dir.path()).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].info.is_err());
        assert!(entries[0].load().is_err());
        entries[0].delete().unwrap();
        drop(entries);
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn crash_writer_process() {
        let Some(root) = std::env::var_os("TYPSTOGRAPHE_TEST_CRASH_RECOVERY") else {
            return;
        };
        let (mut recovery, _) = Recovery::start(Path::new(&root), Default::default()).unwrap();
        checkpoint(
            &mut recovery,
            1,
            &StoredDocument::demo(DocumentType::Courrier),
        );
        // Sortie du processus sans exécuter les destructeurs ni le parcours Fermer.
        std::process::exit(73);
    }

    #[test]
    fn an_abrupt_process_exit_releases_the_lock_and_leaves_a_recoverable_copy() {
        let dir = tempfile::tempdir().unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "recovery::tests::crash_writer_process"])
            .env("TYPSTOGRAPHE_TEST_CRASH_RECOVERY", dir.path())
            .output()
            .unwrap();
        assert_eq!(
            status.status.code(),
            Some(73),
            "{}",
            String::from_utf8_lossy(&status.stderr)
        );
        let entries = scan(dir.path()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].load().unwrap(),
            StoredDocument::demo(DocumentType::Courrier)
        );
    }
}
