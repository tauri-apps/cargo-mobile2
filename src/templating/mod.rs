mod fancy;
mod filter;
mod init;

pub use self::{fancy::*, filter::*, init::*};

use crate::util::{self, Git};
use std::{
    fmt::{self, Display},
    fs, io,
    path::{Path, PathBuf},
};
use thiserror::Error;

// These packs only show in builds using the brainium feature flag, and will
// always be at the top of the list.
static BRAINIUM: &[&str] = &["brainstorm"];

fn platform_pack_dir() -> Result<PathBuf, util::NoHomeDir> {
    util::install_dir().map(|dir| dir.join("templates/platforms"))
}

fn app_pack_dir() -> Result<PathBuf, util::NoHomeDir> {
    util::install_dir().map(|dir| dir.join("templates/apps"))
}

#[derive(Debug, Error)]
pub enum LookupError {
    #[error(transparent)]
    NoHomeDir(util::NoHomeDir),
    #[error("Didn't find {name} template pack at {tried_toml} or {tried}")]
    MissingPack {
        name: String,
        tried_toml: PathBuf,
        tried: PathBuf,
    },
    #[error(transparent)]
    FancyPackParseFailed(FancyPackParseError),
}

#[derive(Clone, Debug)]
pub enum Pack {
    Simple(PathBuf),
    Fancy(FancyPack),
}

impl Pack {
    pub(super) fn lookup(
        dir: impl AsRef<Path>,
        name: impl AsRef<str>,
    ) -> Result<Self, LookupError> {
        fn check_path(name: &str, path: &Path) -> Option<PathBuf> {
            log::info!("checking for template pack \"{}\" at {:?}", name, path);
            if path.exists() {
                log::info!("found template pack \"{}\" at {:?}", name, path);
                Some(path.to_owned())
            } else {
                None
            }
        }

        let path = {
            let dir = dir.as_ref();
            let name = name.as_ref();
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
            let pack = FancyPack::parse(path).map_err(LookupError::FancyPackParseFailed)?;
            Ok(Pack::Fancy(pack))
        } else {
            Ok(Pack::Simple(path))
        }
    }

    pub fn lookup_platform(name: &str) -> Result<Self, LookupError> {
        platform_pack_dir()
            .map_err(LookupError::NoHomeDir)
            .and_then(|dir| Self::lookup(dir, name))
    }

    pub fn lookup_app(name: &str) -> Result<Self, LookupError> {
        app_pack_dir()
            .map_err(LookupError::NoHomeDir)
            .and_then(|dir| Self::lookup(dir, name))
    }

    pub fn expect_local(self) -> PathBuf {
        if let Self::Simple(path) = self {
            path
        } else {
            panic!("developer error: called `expect_local` on a `Pack::Fancy`")
        }
    }

    pub fn submodule_path(&self) -> Option<&Path> {
        if let Self::Fancy(pack) = self {
            pack.submodule_path()
        } else {
            None
        }
    }

    pub fn resolve(
        &self,
        git: Git<'_>,
        submodule_commit: Option<&str>,
    ) -> Result<Vec<&Path>, FancyPackResolveError> {
        match self {
            Self::Simple(path) => {
                if submodule_commit.is_some() {
                    log::warn!(
                        "specified a submodule commit, but the template pack {:?} isn't submodule-based", path
                    );
                }
                Ok(vec![path])
            }
            Self::Fancy(pack) => pack.resolve(git, submodule_commit),
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

pub fn list_app_packs() -> Result<Vec<String>, ListError> {
    let dir = app_pack_dir().map_err(ListError::NoHomeDir)?;
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
            if !BRAINIUM.contains(&name.as_ref()) {
                packs.push(name.into_owned());
            }
        }
    }
    packs.sort_unstable();
    packs.dedup();
    Ok(if cfg!(feature = "brainium") {
        // This solution is slightly devious...
        BRAINIUM
            .iter()
            .map(ToString::to_string)
            .chain(packs)
            .collect()
    } else {
        packs
    })
}
