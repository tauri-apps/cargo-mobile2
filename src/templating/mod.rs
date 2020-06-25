mod filter;
mod init;
mod remote;

pub use self::{filter::*, init::*, remote::*};

use crate::util::{self, Git};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    fs, io,
    path::{Path, PathBuf},
};

// These packs won't be returned by `list_packs`, since they can't/shouldn't be
// used as base projects.
static HIDDEN: &'static [&'static str] = &["android-studio-project", "xcode-project"];

fn pack_dir() -> Result<PathBuf, util::NoHomeDir> {
    util::install_dir().map(|dir| dir.join("templates"))
}

#[derive(Debug)]
pub enum LookupError {
    NoHomeDir(util::NoHomeDir),
    MissingPack {
        name: String,
        tried_toml: PathBuf,
        tried: PathBuf,
    },
    RemotePackParseFailed(RemotePackParseError),
}

impl Display for LookupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoHomeDir(err) => write!(f, "{}", err),
            Self::MissingPack {
                name,
                tried_toml,
                tried,
            } => write!(
                f,
                "Didn't find {:?} template pack at {:?} or {:?}",
                name, tried_toml, tried
            ),
            Self::RemotePackParseFailed(err) => write!(f, "{}", err),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Pack {
    Local(PathBuf),
    Remote(RemotePack),
}

impl Pack {
    pub fn lookup(name: &str) -> Result<Self, LookupError> {
        fn check_path(name: &str, path: &Path) -> Option<PathBuf> {
            log::info!("checking for template pack \"{}\" at {:?}", name, path);
            if path.exists() {
                log::info!("found template pack \"{}\" at {:?}", name, path);
                Some(path.to_owned())
            } else {
                None
            }
        }

        let dir = pack_dir().map_err(LookupError::NoHomeDir)?;
        let path = {
            let toml_path = dir.join(format!("{}.toml", name));
            let path = dir.join(name);
            check_path(name, &toml_path)
                .or_else(|| check_path(name, &path))
                .ok_or_else(|| LookupError::MissingPack {
                    name: name.to_owned(),
                    tried_toml: toml_path,
                    tried: path,
                })
        }?;
        if path.extension() == Some("toml".as_ref()) {
            let pack = RemotePack::parse(path).map_err(LookupError::RemotePackParseFailed)?;
            Ok(Pack::Remote(pack))
        } else {
            Ok(Pack::Local(path))
        }
    }

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
pub enum ListError {
    NoHomeDir(util::NoHomeDir),
    DirReadFailed { dir: PathBuf, cause: io::Error },
    DirEntryReadFailed { dir: PathBuf, cause: io::Error },
}

impl Display for ListError {
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

pub fn list_packs() -> Result<Vec<String>, ListError> {
    let dir = pack_dir().map_err(ListError::NoHomeDir)?;
    let mut packs = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|cause| ListError::DirReadFailed {
        dir: dir.clone(),
        cause,
    })? {
        let entry = entry.map_err(|cause| ListError::DirEntryReadFailed {
            dir: dir.clone(),
            cause,
        })?;
        if let Some(name) = entry.path().file_stem() {
            let name = name.to_string_lossy();
            if !HIDDEN.contains(&name.as_ref()) {
                packs.push(name.into_owned());
            }
        }
    }
    packs.sort_unstable();
    packs.dedup();
    Ok(packs)
}
