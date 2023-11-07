use super::{LookupError, Pack};
use crate::util::{
    self,
    submodule::{self, Submodule},
    Git,
};
use serde::Deserialize;
use std::{
    fs, io,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FancyPackParseError {
    #[error("Failed to read remote template pack spec {path}: {cause}")]
    ReadFailed { path: PathBuf, cause: io::Error },
    #[error("Failed to parse remote template pack spec {path}: {cause}")]
    ParseFailed {
        path: PathBuf,
        cause: toml::de::Error,
    },
    #[error(transparent)]
    NoHomeDir(util::NoHomeDir),
    #[error("Failed to lookup base template pack: {0}")]
    BaseFailed(Box<LookupError>),
}

#[derive(Debug, Error)]
pub enum FancyPackResolveError {
    #[error("Failed to initialize submodule: {0}")]
    SubmoduleFailed(submodule::Error),
    #[error("Template pack wasn't found at {0}")]
    PackNotFound(PathBuf),
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
            let toml_str =
                fs::read_to_string(path).map_err(|cause| FancyPackParseError::ReadFailed {
                    path: path.to_owned(),
                    cause,
                })?;
            toml::from_str::<Raw>(&toml_str).map_err(|cause| FancyPackParseError::ParseFailed {
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
                        name,
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
