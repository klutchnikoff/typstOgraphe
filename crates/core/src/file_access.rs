//! Verrou coopératif stable malgré le remplacement atomique de l’archive.
//! Ne jamais supprimer le compagnon : deux inodes permettraient deux détenteurs.
use crate::protocol::{CommandError, ErrorCode};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions, TryLockError},
    io::{self, Read, Seek},
    path::{Path, PathBuf},
    sync::Arc,
};

pub(crate) type Fingerprint = [u8; 32];

pub(crate) fn fingerprint(file: &mut File) -> io::Result<Fingerprint> {
    file.rewind()?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    // Même borne que les archives : pas de lecture sans limite d’un fichier externe.
    let limit = crate::assets::MAX_DOCUMENT_BYTES + 2 * 1024 * 1024;
    let mut reader = file.take(limit + 1);
    let mut size = 0;
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        size += count as u64;
        if size > limit {
            return Err(io::Error::other("document size limit"));
        }
        hash.update(&buffer[..count]);
    }
    Ok(hash.finalize().into())
}

fn io_error(error: io::Error) -> CommandError {
    CommandError::detail(ErrorCode::Io, "file_access", error)
}

pub(crate) fn canonical_target(path: &Path) -> Result<PathBuf, CommandError> {
    if path.try_exists().map_err(io_error)? {
        return path.canonicalize().map_err(io_error);
    }
    // Un lien pendant ne doit pas créer une deuxième identité de verrou.
    if path.symlink_metadata().is_ok() {
        return Err(CommandError::new(ErrorCode::Io, "file_access"));
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = path
        .file_name()
        .ok_or_else(|| CommandError::new(ErrorCode::InvalidParams, "save_path_required"))?;
    Ok(parent.canonicalize().map_err(io_error)?.join(name))
}

#[derive(Clone)]
struct Binding {
    target: PathBuf,
    expected: Option<Fingerprint>,
    // Arc ne partage que le verrou OS, jamais une session ou une empreinte mutable.
    _lease: Arc<File>,
}

impl Binding {
    fn acquire(path: &Path) -> Result<Self, CommandError> {
        let target = canonical_target(path)?;
        let lease = {
            let mut name = std::ffi::OsString::from(".");
            name.push(target.file_name().expect("canonical file name"));
            name.push(".lock");
            let mut options = OpenOptions::new();
            options.read(true).write(true).create(true).truncate(false);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let file = options
                .open(target.with_file_name(name))
                .map_err(io_error)?;
            file.try_lock().map_err(|error| match error {
                TryLockError::WouldBlock => {
                    CommandError::new(ErrorCode::Unavailable, "file_locked")
                }
                TryLockError::Error(error) => io_error(error),
            })?;
            Arc::new(file)
        };
        Ok(Self {
            target,
            expected: None,
            _lease: lease,
        })
    }

    pub fn observe(&mut self) -> Result<(), CommandError> {
        self.expected = match File::open(&self.target) {
            Ok(mut file) => Some(fingerprint(&mut file).map_err(io_error)?),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(io_error(error)),
        };
        Ok(())
    }

    pub fn check_unchanged(&self) -> Result<(), CommandError> {
        let mut current = self.clone();
        current.observe()?;
        if current.expected != self.expected {
            return Err(CommandError::new(ErrorCode::Document, "file_changed"));
        }
        Ok(())
    }

    pub fn saved(&mut self, fingerprint: Fingerprint) {
        self.expected = Some(fingerprint);
    }
}

/// Garde de fichier réutilisable par les adaptateurs structurés. Le verrou et
/// l'empreinte protègent l'archive pendant son remplacement atomique.
#[derive(Clone)]
pub struct DocumentLease(Binding);
impl DocumentLease {
    pub fn acquire(path: &Path) -> Result<Self, CommandError> {
        let mut binding = Binding::acquire(path)?;
        binding.observe()?;
        Ok(Self(binding))
    }
    pub fn path(&self) -> &Path {
        &self.0.target
    }
    pub fn check(&self) -> Result<(), CommandError> {
        self.0.check_unchanged()
    }
    pub fn record_save(&mut self) -> Result<(), CommandError> {
        self.0.observe()
    }
    pub fn prepare_fingerprint(&self, file: &mut File) -> Result<[u8; 32], CommandError> {
        fingerprint(file).map_err(io_error)
    }
    pub fn record_fingerprint(&mut self, value: [u8; 32]) {
        self.0.saved(value);
    }
}
