use super::{LookupError, Pack};
use crate::util::{
    self,
    submodule::{self, Submodule},
    Git,
};
use serde::Deserialize;
use std::{
    fmt::{self, Display},
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum FancyPackParseError {
    ReadFailed {
        path: PathBuf,
        cause: io::Error,
    },
    ParseFailed {
        path: PathBuf,
        cause: toml::de::Error,
    },
    NoHomeDir(util::NoHomeDir),
    BaseFailed(Box<LookupError>),
}

impl Display for FancyPackParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadFailed { path, cause } => write!(
                f,
                "Failed to read remote template pack spec {:?}: {}",
                path, cause
            ),
            Self::ParseFailed { path, cause } => write!(
                f,
                "Failed to parse remote template pack spec {:?}: {}",
                path, cause
            ),
            Self::NoHomeDir(err) => write!(f, "{}", err),
            Self::BaseFailed(err) => write!(f, "Failed to lookup base template pack: {}", err),
        }
    }
}

#[derive(Debug)]
pub enum FancyPackResolveError {
    SubmoduleFailed(submodule::Error),
    PackNotFound(PathBuf),
}

impl Display for FancyPackResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SubmoduleFailed(err) => write!(f, "Failed to initialize submodule: {}", err),
            Self::PackNotFound(path) => write!(f, "Template pack wasn't found at {:?}", path),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FancyPack {
    path: PathBuf,
    base: Option<Box<Pack>>,
    submodule: Option<Submodule>,
}

impl FancyPack {
    pub fn parse(path: impl AsRef<Path>) -> Result<Self, FancyPackParseError> {
        #[derive(Deserialize)]
        struct Raw {
            path: PathBuf,
            base: Option<String>,
            submodule: Option<Submodule>,
        }

        let path = path.as_ref();
        let raw = {
            let bytes = fs::read(path).map_err(|cause| FancyPackParseError::ReadFailed {
                path: path.to_owned(),
                cause,
            })?;
            toml::from_slice::<Raw>(&bytes).map_err(|cause| FancyPackParseError::ParseFailed {
                path: path.to_owned(),
                cause,
            })?
        };
        let real_path = util::expand_home(&raw.path).map_err(FancyPackParseError::NoHomeDir)?;
        let this = Self {
            path: real_path,
            base: raw
                .base
                .map(|name| {
                    Pack::lookup(
                        path.parent()
                            .expect("developer error: templates dir had no parent"),
                        &name,
                    )
                })
                .transpose()
                .map_err(Box::new)
                .map_err(FancyPackParseError::BaseFailed)?
                .map(Box::new),
            submodule: raw.submodule,
        };
        log::info!("template pack {:#?}", this);
        Ok(this)
    }

    pub fn submodule_path(&self) -> Option<&Path> {
        self.submodule.as_ref().map(|submodule| submodule.path())
    }

    pub fn resolve(
        &self,
        git: Git<'_>,
        submodule_commit: Option<&str>,
    ) -> Result<Vec<&Path>, FancyPackResolveError> {
        if let Some(submodule) = &self.submodule {
            submodule
                .init(git, submodule_commit)
                .map_err(FancyPackResolveError::SubmoduleFailed)?;
        }
        if self.path.exists() {
            let mut paths = self
                .base
                .as_ref()
                .map(|base| {
                    base.resolve(
                        git,
                        submodule_commit.filter(|_| base.submodule_path() == self.submodule_path()),
                    )
                })
                .transpose()?
                .unwrap_or_default();
            paths.push(&self.path);
            Ok(paths)
        } else {
            Err(FancyPackResolveError::PackNotFound(self.path.clone()))
        }
    }
}
