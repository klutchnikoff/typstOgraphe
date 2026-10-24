//! Private, short-lived rendezvous; never store the token in command-line arguments.
#[cfg(unix)]
use interprocess::local_socket::{GenericFilePath, ToFsName};
#[cfg(windows)]
use interprocess::local_socket::{GenericNamespaced, ToNsName};
use interprocess::local_socket::{ListenerOptions, Name};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Endpoint {
    pub version: u32,
    pub address: String,
    pub token: String,
}
impl Endpoint {
    pub fn name(&self) -> io::Result<Name<'_>> {
        #[cfg(unix)]
        {
            self.address.as_str().to_fs_name::<GenericFilePath>()
        }
        #[cfg(windows)]
        {
            self.address.as_str().to_ns_name::<GenericNamespaced>()
        }
    }
    pub fn listener(&self) -> io::Result<interprocess::local_socket::tokio::Listener> {
        let options = ListenerOptions::new().name(self.name()?);
        #[cfg(windows)]
        let options = {
            use interprocess::os::windows::{
                local_socket::ListenerOptionsExt, security_descriptor::SecurityDescriptor,
            };
            // The owner is assigned by Windows when creating the pipe.
            let sd = SecurityDescriptor::deserialize(widestring::u16cstr!("D:P(A;;GA;;;OW)"))?;
            options.security_descriptor(sd)
        };
        options.create_tokio()
    }
}
pub(super) struct Invitation {
    _directory: tempfile::TempDir,
    path: PathBuf,
    pub data: Endpoint,
}
impl Invitation {
    pub fn new() -> io::Result<Self> {
        // macOS sockaddr_un paths are short; avoid the long per-user TMPDIR.
        #[cfg(unix)]
        let directory = {
            use std::os::unix::fs::PermissionsExt;
            tempfile::Builder::new()
                .prefix("tog-")
                .permissions(fs::Permissions::from_mode(0o700))
                .tempdir_in("/tmp")?
        };
        #[cfg(windows)]
        let directory = tempfile::Builder::new().prefix("tog-").tempdir()?;
        #[cfg(unix)]
        let address = directory.path().join("s").to_string_lossy().into_owned();
        #[cfg(windows)]
        let address = format!("typstographe-{}", uuid::Uuid::new_v4());
        let data = Endpoint {
            version: 1,
            address,
            token: format!(
                "{}{}",
                uuid::Uuid::new_v4().simple(),
                uuid::Uuid::new_v4().simple()
            ),
        };
        let path = directory.path().join("invitation.json");
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        serde_json::to_writer(options.open(&path)?, &data)?;
        Ok(Self {
            _directory: directory,
            path,
            data,
        })
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
}
pub(super) fn read(path: &Path) -> io::Result<Endpoint> {
    use std::io::Read;
    let mut data = Vec::new();
    fs::File::open(path)?.take(4097).read_to_end(&mut data)?;
    if data.len() > 4096 {
        return Err(io::Error::other("Invalid invitation"));
    }
    let endpoint: Endpoint = serde_json::from_slice(&data).map_err(io::Error::other)?;
    if endpoint.version != 1 || endpoint.token.len() != 64 {
        return Err(io::Error::other("Unsupported invitation"));
    }
    Ok(endpoint)
}
