//! Isolated storage probe, not a production .typsto reader or snapshot API.
use git2::{Buf, Oid, Repository, Signature, Time};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs,
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
    time::Instant,
};
use tempfile::TempDir;
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
type Files = BTreeMap<String, Vec<u8>>;
fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn read_zip(bytes: &[u8]) -> Result<Files> {
    let mut zip = ZipArchive::new(Cursor::new(bytes))?;
    let mut files = Files::new();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let path = entry.name()?.into_owned();
        if files.contains_key(&path) {
            return Err("duplicate ZIP entry".into());
        }
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        files.insert(path, data);
    }
    Ok(files)
}
fn zip_to(writer: impl Write + std::io::Seek, files: &Files) -> Result<()> {
    let mut zip = ZipWriter::new(writer);
    for (path, data) in files {
        let method = if path.ends_with(".pack") {
            zip::CompressionMethod::Stored
        } else {
            zip::CompressionMethod::Deflated
        };
        zip.start_file(
            path,
            SimpleFileOptions::default().compression_method(method),
        )?;
        zip.write_all(data)?;
    }
    zip.finish()?;
    Ok(())
}
fn zip_bytes(files: &Files) -> Result<Vec<u8>> {
    let mut output = Cursor::new(Vec::new());
    zip_to(&mut output, files)?;
    Ok(output.into_inner())
}
fn entry_bytes<R: Read + std::io::Seek>(zip: &mut ZipArchive<R>, name: &str) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    zip.by_name(name)?.read_to_end(&mut bytes)?;
    Ok(bytes)
}
// Used by both backends; all writes replace a single archive atomically.
fn publish(path: &Path, files: &Files, fail_before_commit: bool) -> Result<()> {
    let mut temporary = atomic_write_file::AtomicWriteFile::open(path)?;
    zip_to(&mut temporary, files)?;
    if fail_before_commit {
        return Err("injected failure before commit".into());
    }
    temporary.commit()?;
    Ok(())
}
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum Backend {
    Git,
    Cas,
}
impl Backend {
    fn name(self) -> &'static str {
        match self {
            Self::Git => "git",
            Self::Cas => "cas",
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Record {
    id: String,
    created_at: String,
    name: String,
    tags: Vec<String>,
    author: String,
    entries: BTreeMap<String, String>,
    resources: Option<String>,
    commit: Option<String>,
}
impl Record {
    fn new(i: usize, files: &Files) -> Self {
        Self {
            id: format!("{i:03}"),
            created_at: format!("2026-10-31T12:{:02}:{:02}Z", i / 60, i % 60),
            name: format!("Version {i}"),
            tags: vec!["evaluation".into()],
            author: "Storage evaluation".into(),
            entries: files.iter().map(|(p, b)| (p.clone(), hash(b))).collect(),
            resources: None,
            commit: None,
        }
    }
}
struct History {
    backend: Backend,
    // libgit2 operates on an isolated bare object database, never the user's repository.
    _directory: TempDir,
    repo: Option<Repository>,
    records: Vec<Record>,
    objects: Files,
    source: Option<RefCell<ZipArchive<fs::File>>>,
}
impl History {
    fn new(backend: Backend) -> Result<Self> {
        let directory = tempfile::tempdir()?;
        let repo = match backend {
            Backend::Git => Some(Repository::init_bare(directory.path())?),
            Backend::Cas => None,
        };
        Ok(Self {
            backend,
            _directory: directory,
            repo,
            records: vec![],
            objects: Files::new(),
            source: None,
        })
    }
    fn append(&mut self, i: usize, files: &Files) -> Result<()> {
        let mut record = Record::new(i, files);
        if self.records.iter().any(|r| r.id == record.id) {
            return Err("duplicate snapshot".into());
        }
        let mut tree = self
            .repo
            .as_ref()
            .map(|r| r.treebuilder(None))
            .transpose()?;
        for (path, data) in files {
            // Images, bibliography and package files share the same SHA-256 store
            // in both solutions. Git owns the versioned root document files only.
            if let Some(tree) = &mut tree
                && !path.contains('/')
            {
                tree.insert(path, self.repo.as_ref().unwrap().blob(data)?, 0o100644)?;
            } else {
                self.objects
                    .entry(record.entries[path].clone())
                    .or_insert_with(|| data.clone());
            }
        }
        // Keep the timeline compact: identical resource manifests are shared,
        // rather than copying hundreds of package paths into every index row.
        let resources: BTreeMap<_, _> = record
            .entries
            .iter()
            .filter(|(p, _)| p.contains('/'))
            .map(|(p, h)| (p.clone(), h.clone()))
            .collect();
        if !resources.is_empty() {
            let bytes = serde_json::to_vec(&resources)?;
            let digest = hash(&bytes);
            self.objects.entry(digest.clone()).or_insert(bytes);
            record.resources = Some(digest);
            record.entries.retain(|p, _| !p.contains('/'));
        }
        if let Some(tree) = tree {
            let repo = self.repo.as_ref().unwrap();
            let root = repo.find_tree(tree.write()?)?;
            let signature = Signature::new(
                "Evaluation",
                "evaluation@invalid.example",
                &Time::new(1_798_646_400 + i as i64, 0),
            )?;
            // Independent commits allow deletion without retaining an ancestor
            // through a later commit; the application index defines the timeline.
            record.commit = Some(
                repo.commit(None, &signature, &signature, &record.name, &root, &[])?
                    .to_string(),
            );
        }
        self.records.push(record);
        Ok(())
    }
    fn envelope(&self) -> Result<Files> {
        let mut files = Files::new();
        files.insert(
            "history/index.json".into(),
            serde_json::to_vec(&self.records)?,
        );
        let needed = self.needed_objects()?;
        for digest in needed {
            files.insert(format!("history/objects/{digest}"), self.object(&digest)?);
        }
        if let Some(repo) = &self.repo {
            let mut builder = repo.packbuilder()?;
            builder.set_threads(1); // Reproducible CPU policy, no machine-wide parallel pack.
            for record in self.records.iter().rev() {
                builder.insert_commit(Oid::from_str(
                    record.commit.as_ref().ok_or("missing commit")?,
                )?)?;
            }
            let mut pack = Buf::new();
            builder.write_buf(&mut pack)?;
            files.insert("history/git.pack".into(), pack.to_vec());
        }
        Ok(files)
    }
    fn entries_for(&self, record: &Record) -> Result<BTreeMap<String, String>> {
        let mut entries = record.entries.clone();
        if let Some(digest) = &record.resources {
            for (path, digest) in self.resource_entries(digest)? {
                if entries.insert(path, digest).is_some() {
                    return Err("invalid resource manifest".into());
                }
            }
        }
        Ok(entries)
    }
    fn resource_entries(&self, digest: &str) -> Result<BTreeMap<String, String>> {
        let bytes = self.object(digest)?;
        if hash(&bytes) != digest {
            return Err("resource manifest integrity mismatch".into());
        }
        let entries: BTreeMap<String, String> = serde_json::from_slice(&bytes)?;
        if entries.keys().any(|p| !p.contains('/')) {
            return Err("invalid resource manifest".into());
        }
        Ok(entries)
    }
    fn needed_objects(&self) -> Result<BTreeSet<String>> {
        let mut needed = BTreeSet::new();
        let mut manifests = BTreeSet::new();
        for record in &self.records {
            if let Some(digest) = &record.resources {
                manifests.insert(digest);
            }
            for (path, digest) in &record.entries {
                if self.repo.is_none() || path.contains('/') {
                    needed.insert(digest.clone());
                }
            }
        }
        for digest in manifests {
            needed.insert(digest.clone());
            needed.extend(self.resource_entries(digest)?.into_values());
        }
        Ok(needed)
    }
    fn save(&self, path: &Path) -> Result<()> {
        publish(path, &self.envelope()?, false)
    }
    fn open(backend: Backend, path: &Path) -> Result<Self> {
        let mut source = ZipArchive::new(fs::File::open(path)?)?;
        let mut essential = Files::new();
        essential.insert(
            "history/index.json".into(),
            entry_bytes(&mut source, "history/index.json")?,
        );
        if matches!(backend, Backend::Git) {
            essential.insert(
                "history/git.pack".into(),
                entry_bytes(&mut source, "history/git.pack")?,
            );
        }
        let mut result = Self::from_envelope(backend, essential)?;
        result.source = Some(RefCell::new(source));
        Ok(result)
    }
    fn object(&self, digest: &str) -> Result<Vec<u8>> {
        if let Some(bytes) = self.objects.get(digest) {
            return Ok(bytes.clone());
        }
        let source = self.source.as_ref().ok_or("missing object")?;
        entry_bytes(
            &mut source.borrow_mut(),
            &format!("history/objects/{digest}"),
        )
    }
    fn object_keys(&self) -> Result<BTreeSet<String>> {
        let mut keys: BTreeSet<_> = self.objects.keys().cloned().collect();
        if let Some(source) = &self.source {
            for name in source.borrow().file_names() {
                if let Some(key) = name?.strip_prefix("history/objects/") {
                    keys.insert(key.into());
                }
            }
        }
        Ok(keys)
    }
    fn from_envelope(backend: Backend, files: Files) -> Result<Self> {
        let mut result = Self::new(backend)?;
        result.records =
            serde_json::from_slice(files.get("history/index.json").ok_or("missing index")?)?;
        let ids: BTreeSet<_> = result.records.iter().map(|r| &r.id).collect();
        if ids.len() != result.records.len() {
            return Err("duplicate snapshot ID".into());
        }
        result.objects = files
            .iter()
            .filter_map(|(p, b)| {
                p.strip_prefix("history/objects/")
                    .map(|digest| (digest.into(), b.clone()))
            })
            .collect();
        if let Some(repo) = &result.repo {
            let odb = repo.odb()?;
            let mut writer = odb.packwriter()?;
            writer.write_all(files.get("history/git.pack").ok_or("missing pack")?)?;
            writer.commit()?;
        }
        Ok(result)
    }
    fn restore(&self, id: &str) -> Result<Files> {
        let record = self
            .records
            .iter()
            .find(|r| r.id == id)
            .ok_or("unknown snapshot")?;
        let tree = self
            .repo
            .as_ref()
            .map(|repo| -> Result<_> {
                Ok(repo
                    .find_commit(Oid::from_str(
                        record.commit.as_ref().ok_or("missing commit")?,
                    )?)?
                    .tree()?)
            })
            .transpose()?;
        let mut files = Files::new();
        for (path, expected) in self.entries_for(record)? {
            let data = if let Some(tree) = &tree
                && !path.contains('/')
            {
                let entry = tree.get_name(&path).ok_or("missing tree entry")?;
                self.repo
                    .as_ref()
                    .unwrap()
                    .find_blob(entry.id())?
                    .content()
                    .to_vec()
            } else {
                self.object(&expected)?
            };
            if hash(&data) != expected {
                return Err("object integrity mismatch".into());
            }
            files.insert(path, data);
        }
        Ok(files)
    }
    fn retain(&mut self, ids: &[&str]) {
        self.records.retain(|r| ids.contains(&r.id.as_str()));
    }
}
#[derive(Serialize)]
struct Timing {
    samples_ms: Vec<f64>,
    median_ms: f64,
    p95_ms: f64,
}
fn elapsed<T>(f: impl FnOnce() -> Result<T>) -> Result<(T, f64)> {
    let start = Instant::now();
    let value = f()?;
    Ok((value, start.elapsed().as_secs_f64() * 1000.0))
}
fn summary(samples: Vec<f64>) -> Timing {
    let mut sorted = samples.clone();
    sorted.sort_by(f64::total_cmp);
    Timing {
        median_ms: sorted[sorted.len() / 2],
        p95_ms: sorted[(sorted.len() * 95).div_ceil(100).saturating_sub(1)],
        samples_ms: samples,
    }
}
#[derive(Deserialize)]
struct Corpus {
    name: String,
    directory: PathBuf,
    count: usize,
}
#[derive(Serialize)]
struct Measurement {
    corpus: String,
    backend: Backend,
    snapshots: usize,
    archive_bytes: u64,
    pruned_archive_bytes: u64,
    current_project_bytes: u64,
    distinct_objects: usize,
    package_entries: usize,
    create_first: Timing,
    add_and_save: Timing,
    open_and_restore: Timing,
    restore_warm: Timing,
    prune_and_save: Timing,
    first_store_build_ms: f64,
}
fn assert_equal(actual: &Files, expected: &Files) -> Result<()> {
    if actual != expected {
        return Err("restored snapshot bytes differ".into());
    }
    Ok(())
}
fn verify_failures(history: &History, target: &Path, id: &str) -> Result<usize> {
    let expected = history.restore(id)?;
    let before = fs::read(target)?;
    let mut candidate = history.envelope()?;
    candidate.insert(
        "history/incomplete-candidate".into(),
        b"must not be published".to_vec(),
    );
    if publish(target, &candidate, true).is_ok() {
        return Err("injected failure not detected".into());
    }
    if fs::read(target)? != before {
        return Err("failed publication replaced old archive".into());
    }
    assert_equal(
        &History::open(history.backend, target)?.restore(id)?,
        &expected,
    )?;
    let mut files = history.envelope()?;
    // A ZIP-valid but damaged content object must fail on read, not silently change content.
    let entries = history.entries_for(history.records.iter().find(|r| r.id == id).unwrap())?;
    let object = entries
        .iter()
        .find(|(p, _)| history.repo.is_none() || p.contains('/'))
        .map(|(_, h)| h)
        .unwrap();
    let key = format!("history/objects/{object}");
    let bytes = files.get_mut(&key).unwrap();
    bytes[0] ^= 1;
    let damaged = History::from_envelope(history.backend, files)?;
    if damaged.restore(id).is_ok() {
        return Err("damaged object accepted".into());
    }
    let mut files = history.envelope()?;
    files.remove(&key);
    let incomplete = History::from_envelope(history.backend, files)?;
    if incomplete.restore(id).is_ok() {
        return Err("missing resource accepted".into());
    }
    let truncated = &before[..before.len() / 2];
    if read_zip(truncated).is_ok() {
        return Err("truncated archive accepted".into());
    }
    let mut checks = 4;
    if matches!(history.backend, Backend::Git) {
        let mut files = history.envelope()?;
        let pack = files.get_mut("history/git.pack").unwrap();
        let middle = pack.len() / 2;
        pack[middle] ^= 1;
        if History::from_envelope(history.backend, files)
            .and_then(|h| h.restore(id))
            .is_ok()
        {
            return Err("damaged Git pack accepted".into());
        }
        checks += 1;
    }
    Ok(checks)
}
fn main() -> Result<()> {
    let output = PathBuf::from(std::env::args().nth(1).ok_or("expected corpus directory")?);
    let corpus_metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join("corpus.json"))?)?;
    let corpora: Vec<Corpus> = serde_json::from_value(corpus_metadata["corpora"].clone())?;
    fs::create_dir_all(output.join("restored"))?;
    let scratch = tempfile::tempdir_in(&output)?;
    let mut measurements = vec![];
    let mut verified_states = 0;
    let mut failure_checks = 0;
    for corpus in &corpora {
        let fixtures: Vec<Files> = (0..corpus.count)
            .map(|i| read_zip(&fs::read(corpus.directory.join(format!("{i:03}.typsto")))?))
            .collect::<Result<_>>()?;
        for count in [20, 100] {
            for backend in [Backend::Git, Backend::Cas] {
                eprintln!("{} / {} / {count} snapshots", corpus.name, backend.name());
                let path = scratch.path().join("history.store");
                let mut add = vec![];
                let mut first = vec![];
                let mut open = vec![];
                let mut restore = vec![];
                let mut prune = vec![];
                let mut archive_bytes = 0;
                let mut pruned_bytes = 0;
                let mut object_count = 0;
                let mut package_entries = 0;
                let mut first_store_build_ms = 0.0;
                for sample in 0..8 {
                    // First sample warms compression, filesystem and library paths.
                    let (_, first_ms) = elapsed(|| {
                        let mut initial = History::new(backend)?;
                        initial.append(0, &fixtures[0])?;
                        initial.save(&path)
                    })?;
                    let (mut history, build_ms) = elapsed(|| {
                        let mut h = History::new(backend)?;
                        for (i, files) in fixtures.iter().enumerate().take(count - 1) {
                            h.append(i, files)?;
                        }
                        Ok(h)
                    })?;
                    if sample == 0 {
                        first_store_build_ms = build_ms;
                    }
                    // Existing durable history: final capture + full atomic archive replacement.
                    history.save(&path)?;
                    let (_, add_ms) = elapsed(|| {
                        history.append(count - 1, &fixtures[count - 1])?;
                        history.save(&path)
                    })?;
                    archive_bytes = fs::metadata(&path)?.len();
                    let ((mut opened, restored), open_ms) = elapsed(|| {
                        let h = History::open(backend, &path)?;
                        let files = h.restore("000")?;
                        Ok((h, files))
                    })?;
                    if serde_json::to_vec(&opened.records)? != serde_json::to_vec(&history.records)?
                    {
                        return Err("snapshot metadata changed during reopening".into());
                    }
                    assert_equal(&restored, &fixtures[0])?;
                    let (_, restore_ms) = elapsed(|| opened.restore(&format!("{:03}", count / 2)))?;
                    if sample == 0 {
                        for (i, expected) in fixtures.iter().enumerate().take(count) {
                            assert_equal(&opened.restore(&format!("{i:03}"))?, expected)?;
                            verified_states += 1;
                        }
                        object_count = opened.object_keys()?.len();
                        package_entries = opened.envelope()?.len();
                        failure_checks += verify_failures(&opened, &path, "000")?;
                        if count == 100 {
                            for id in ["000", "010", "020", "030", "099"] {
                                let restored = opened.restore(id)?;
                                let destination = output.join("restored").join(format!(
                                    "{}-{}-{id}.typsto",
                                    corpus.name,
                                    backend.name()
                                ));
                                fs::write(destination, zip_bytes(&restored)?)?;
                            }
                        }
                    }
                    // Delete intermediate snapshots, retain the oldest + current.
                    let last = format!("{:03}", count - 1);
                    let (_, prune_ms) = elapsed(|| {
                        opened.retain(&["000", &last]);
                        opened.save(&path)
                    })?;
                    pruned_bytes = fs::metadata(&path)?.len();
                    let pruned = History::open(backend, &path)?;
                    if pruned.records.len() != 2 {
                        return Err("prune count mismatch".into());
                    }
                    assert_equal(&pruned.restore("000")?, &fixtures[0])?;
                    assert_equal(&pruned.restore(&last)?, &fixtures[count - 1])?;
                    if pruned.restore("010").is_ok() {
                        return Err("deleted snapshot still listed".into());
                    }
                    // No orphan binary objects remain in the persisted envelope.
                    let reachable = pruned.needed_objects()?;
                    if pruned.object_keys()?.iter().any(|h| !reachable.contains(h)) {
                        return Err("orphan object retained".into());
                    }
                    if sample > 0 {
                        first.push(first_ms);
                        add.push(add_ms);
                        open.push(open_ms);
                        restore.push(restore_ms);
                        prune.push(prune_ms);
                    }
                }
                measurements.push(Measurement {
                    corpus: corpus.name.clone(),
                    backend,
                    snapshots: count,
                    archive_bytes,
                    pruned_archive_bytes: pruned_bytes,
                    current_project_bytes: fs::metadata(
                        corpus.directory.join(format!("{:03}.typsto", count - 1)),
                    )?
                    .len(),
                    distinct_objects: object_count,
                    package_entries,
                    first_store_build_ms,
                    create_first: summary(first),
                    add_and_save: summary(add),
                    open_and_restore: summary(open),
                    restore_warm: summary(restore),
                    prune_and_save: summary(prune),
                });
            }
        }
    }
    let version = git2::Version::get();
    let report = serde_json::json!({
        "schema": 3, "environment": corpus_metadata,
        "libgit2": {"version":version.libgit2_version(), "vendored":version.vendored(), "https":version.https(), "ssh":version.ssh()},
        "policy": {"measured_samples":7,"warmup_samples":1,"git_pack_threads":1,"atomic_replacement":true,"zip_deflate":true,"git_pack_stored_without_recompression":true,"timing_excludes_fixture_generation_core_validation_and_typst":true,"warm_filesystem_cache":true,"selective_resource_loading":true,"deduplicated_resource_manifests":true},
        "checks": {"byte_exact_snapshots":verified_states,"failure_checks":failure_checks,"prune_preserves_retained_states":true,"prune_removes_orphan_binary_objects":true},
        "measurements":measurements,
    });
    fs::write(
        output.join("results.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(i: usize) -> Files {
        BTreeMap::from([
            (
                "document.json".into(),
                format!("{{\"revision\":{i}}}").into_bytes(),
            ),
            ("state.yjs".into(), vec![i as u8; 100]),
            ("images/shared.png".into(), vec![1, 2, 3]),
            (format!("images/unique-{i}.png"), vec![i as u8 + 1; 100]),
        ])
    }
    #[test]
    fn both_engines_preserve_selected_states_through_pruning_and_detect_failed_publication()
    -> Result<()> {
        for backend in [Backend::Git, Backend::Cas] {
            let directory = tempfile::tempdir()?;
            let path = directory.path().join("history.store");
            let mut history = History::new(backend)?;
            for i in 0..4 {
                history.append(i, &fixture(i))?;
            }
            history.save(&path)?;
            verify_failures(&history, &path, "000")?;
            let mut opened = History::open(backend, &path)?;
            assert!(
                opened.objects.is_empty(),
                "opening must not eagerly decompress objects"
            );
            opened.retain(&["000", "003"]);
            opened.save(&path)?;
            let restored = History::open(backend, &path)?;
            assert_equal(&restored.restore("000")?, &fixture(0))?;
            assert_equal(&restored.restore("003")?, &fixture(3))?;
            assert!(restored.restore("001").is_err());
            assert!(!restored.object_keys()?.contains(&hash(&[2; 100])));
        }
        Ok(())
    }
}
