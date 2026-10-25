//! Private advertisements of explicitly authorized, not-yet-connected invitations.
//! The OS lock is the liveness signal; no PID guessing, tokens or document paths.
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

const LIMIT: u64 = 4096;
const MAX_ENTRIES: usize = 1024;
pub const DIRECTORY_ENV: &str = "TYPSTOGRAPHE_MCP_DIRECTORY";

/// Shared by the GUI and bridge, without a dependency on egui or preference storage.
/// The override is useful for isolated diagnostics; both processes must use it.
pub fn directory() -> io::Result<PathBuf> {
    if let Some(path) = std::env::var_os(DIRECTORY_ENV) {
        let path = PathBuf::from(path);
        if path.is_absolute() {
            return Ok(path);
        }
        return Err(io::Error::other("MCP discovery directory must be absolute"));
    }
    home::home_dir()
        .filter(|p| p.is_absolute())
        .map(|p| p.join(".typstographe").join("mcp"))
        .ok_or_else(|| io::Error::other("Cannot locate the private MCP discovery directory"))
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    version: u32,
    invitation: PathBuf,
}

pub(super) struct Advertisement {
    record: PathBuf,
    lease: PathBuf,
    lock: Option<File>,
}
fn private_directory(path: &Path) -> io::Result<()> {
    if !path.is_absolute() {
        return Err(io::Error::other("MCP discovery directory must be absolute"));
    }
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)?;
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(io::Error::other("Invalid MCP discovery directory"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o077 != 0 {
            return Err(io::Error::other(
                "MCP discovery directory must be private (0700)",
            ));
        }
    }
    Ok(())
}
impl Advertisement {
    pub fn publish(root: &Path, invitation: &Path) -> io::Result<Self> {
        private_directory(root)?;
        let id = uuid::Uuid::new_v4();
        let record = root.join(format!("{id}.json"));
        let lease = root.join(format!("{id}.lock"));
        let mut options = File::options();
        options.read(true).write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = options.open(&lease)?;
        let advertisement = Self {
            record,
            lease,
            lock: Some(lock),
        };
        advertisement
            .lock
            .as_ref()
            .unwrap()
            .try_lock()
            .map_err(io::Error::other)?;
        let mut options = atomic_write_file::AtomicWriteFile::options();
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&advertisement.record)?;
        file.write_all(&serde_json::to_vec(&Record {
            version: 1,
            invitation: invitation.to_path_buf(),
        })?)?;
        file.commit()?;
        Ok(advertisement)
    }
}
impl Drop for Advertisement {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.record);
        // Windows must close the handle before removing its companion file.
        self.lock.take();
        let _ = fs::remove_file(&self.lease);
    }
}

/// Resolve exactly one live advertisement. Never choose the newest or fall back
/// to standalone access; connection/authentication remains the existing handshake.
pub fn resolve(root: &Path) -> io::Result<PathBuf> {
    if !root.is_absolute() {
        return Err(io::Error::other("MCP discovery directory must be absolute"));
    }
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Err(none()),
        Err(e) => return Err(e),
    };
    private_directory(root)?;
    let mut found = None;
    for (index, entry) in entries.enumerate() {
        if index >= MAX_ENTRIES {
            return Err(io::Error::other("Too many MCP discovery entries"));
        }
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let Some(id) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if uuid::Uuid::parse_str(id).is_err() {
            continue;
        }
        if !entry.file_type()?.is_file() {
            return Err(io::Error::other("Invalid MCP advertisement"));
        }
        let lease = path.with_extension("lock");
        let meta = match fs::symlink_metadata(&lease) {
            Ok(meta) => meta,
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e),
        };
        if !meta.is_file() || meta.file_type().is_symlink() {
            return Err(io::Error::other("Invalid MCP lease"));
        }
        let file = File::options().read(true).write(true).open(&lease)?;
        match file.try_lock() {
            Ok(()) => continue, // Crashed/ended publisher; never use stale data.
            Err(std::fs::TryLockError::WouldBlock) => {}
            Err(std::fs::TryLockError::Error(e)) => return Err(e),
        }
        let mut bytes = Vec::new();
        File::open(&path)?.take(LIMIT + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > LIMIT {
            return Err(io::Error::other("Invalid MCP advertisement size"));
        }
        let record: Record = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
        if record.version != 1 || !record.invitation.is_absolute() {
            return Err(io::Error::other("Invalid MCP advertisement"));
        }
        if found.replace(record.invitation).is_some() {
            return Err(io::Error::other(
                "Several invitations are available. Revoke the others in typstOgraphe before reconnecting, or use --attach INVITATION.",
            ));
        }
    }
    found.ok_or_else(none)
}
fn none() -> io::Error {
    io::Error::other(
        "No invitation is available. Open a document in typstOgraphe, authorize agent assistance, then explicitly reconnect the MCP server.",
    )
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
#[path = "discovery_tests.rs"]
mod tests;
