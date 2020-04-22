mod init;
mod remote;

pub use self::{init::*, remote::*};

use crate::util::{self, Git};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct MissingPack {
    name: String,
    tried_toml: PathBuf,
    tried: PathBuf,
}

impl Display for MissingPack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Didn't find {:?} template pack at {:?} or {:?}",
            self.name, self.tried_toml, self.tried
        )
    }
}

pub fn find_pack(dir: impl AsRef<Path>, name: &str) -> Result<PathBuf, MissingPack> {
    fn check_path(name: &str, path: &Path) -> Option<PathBuf> {
        log::info!("checking for template pack \"{}\" at {:?}", name, path);
        if path.exists() {
            log::info!("found template pack \"{}\" at {:?}", name, path);
            Some(path.to_owned())
        } else {
            None
        }
    }
    let toml_path = dir.as_ref().join(format!("{}.toml", name));
    let path = dir.as_ref().join(name);
    check_path(name, &toml_path)
        .or_else(|| check_path(name, &path))
        .ok_or_else(|| MissingPack {
            name: name.to_owned(),
            tried_toml: toml_path,
            tried: path,
        })
}

fn bundled_pack_dir() -> Result<PathBuf, util::NoHomeDir> {
    util::home_dir().map(|home| home.join(concat!(".", env!("CARGO_PKG_NAME"), "/templates")))
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Pack {
    Local(PathBuf),
    Remote(RemotePack),
}

impl Pack {
    pub fn expect_local(self) -> PathBuf {
        if let Self::Local(path) = self {
            path
        } else {
            panic!("developer error: called `expect_local` on a `Pack::Remote`")
        }
    }

    pub fn submodule_path(&self) -> Option<&Path> {
        if let Self::Remote(pack) = self {
            pack.submodule_path()
        } else {
            None
        }
    }

    pub fn resolve(&self, git: Git<'_>) -> Result<&Path, RemotePackResolveError> {
        match self {
            Self::Local(path) => Ok(&path),
            Self::Remote(pack) => pack.resolve(git),
        }
    }
}

#[derive(Debug)]
pub enum BundledPackError {
    NoHomeDir(util::NoHomeDir),
    MissingPack(MissingPack),
    RemotePackParseFailed(RemotePackParseError),
}

impl Display for BundledPackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoHomeDir(err) => write!(f, "{}", err),
            Self::MissingPack(err) => write!(f, "{}", err),
            Self::RemotePackParseFailed(err) => write!(f, "{}", err),
        }
    }
}

pub fn bundled_pack(name: &str) -> Result<Pack, BundledPackError> {
    let dir = bundled_pack_dir().map_err(BundledPackError::NoHomeDir)?;
    let path = find_pack(dir, name).map_err(BundledPackError::MissingPack)?;
    if path.extension() == Some("toml".as_ref()) {
        let pack = RemotePack::parse(path).map_err(BundledPackError::RemotePackParseFailed)?;
        Ok(Pack::Remote(pack))
    } else {
        Ok(Pack::Local(path))
    }
}

#[derive(Debug)]
pub enum ListBundledPackError {
    NoHomeDir(util::NoHomeDir),
    DirReadFailed { dir: PathBuf, cause: io::Error },
    DirEntryReadFailed { dir: PathBuf, cause: io::Error },
}

impl Display for ListBundledPackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoHomeDir(err) => write!(f, "{}", err),
            Self::DirReadFailed { dir, cause } => {
                write!(f, "Failed to read directory {:?}: {}", dir, cause)
            }
            Self::DirEntryReadFailed { dir, cause } => {
                write!(f, "Failed to read entry in directory {:?}: {}", dir, cause)
            }
        }
    }
}

pub fn list_bundled_packs() -> Result<Vec<String>, ListBundledPackError> {
    let dir = bundled_pack_dir().map_err(ListBundledPackError::NoHomeDir)?;
    let mut packs = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|cause| ListBundledPackError::DirReadFailed {
        dir: dir.clone(),
        cause,
    })? {
        let entry = entry.map_err(|cause| ListBundledPackError::DirEntryReadFailed {
            dir: dir.clone(),
            cause,
        })?;
        if let Some(name) = entry.path().file_stem() {
            if name != "android-studio-project" && name != "xcode-project" {
                packs.push(name.to_string_lossy().into_owned());
            }
        }
    }
    packs.sort_unstable();
    packs.dedup();
    Ok(packs)
}
