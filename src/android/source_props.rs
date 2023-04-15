use crate::util;
use once_cell_regex::regex;
use std::{
    collections::HashMap,
    fmt::Display,
    path::{Path, PathBuf},
};
use thiserror::Error;

type Props = HashMap<String, String>;

#[derive(Debug, Error)]
pub enum RevisionError {
    #[error("Failed to match regex in string {revision:?}")]
    SearchFailed { revision: String },
    #[error(transparent)]
    TripleInvalid(#[from] util::VersionTripleError),
    #[error("Failed to parse beta version from {revision:?}: {source}")]
    BetaInvalid {
        revision: String,
        source: std::num::ParseIntError,
    },
}

#[derive(Debug, Default)]
pub struct Revision {
    pub triple: util::VersionTriple,
    pub beta: Option<u32>,
}

impl Display for Revision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.triple)?;
        if let Some(beta) = self.beta {
            write!(f, "-beta{}", beta)?;
        }
        Ok(())
    }
}

impl Revision {
    fn from_str(revision: &str) -> Result<Self, RevisionError> {
        // Referenced from `$NDK_HOME/build/cmake/android.toolchain.cmake`
        let caps = regex!(r"(?P<version>(?P<major>[0-9]+)\.(?P<minor>[0-9]+)\.(?P<patch>[0-9]+)(-beta(?P<beta>[0-9]+))?)")
            .captures(revision)
            .ok_or_else(|| RevisionError::SearchFailed { revision: revision.to_owned()})?;
        let (triple, version_str) = util::VersionTriple::from_caps(&caps)?;
        Ok(Self {
            triple,
            beta: caps
                .name("beta")
                .map(|beta| beta.as_str().parse())
                .transpose()
                .map_err(|source| RevisionError::BetaInvalid {
                    revision: version_str.to_owned(),
                    source,
                })?,
        })
    }
}

#[derive(Debug, Error)]
pub enum PkgError {
    #[error("`Pkg.Revision` missing.")]
    RevisionMissing,
    #[error("Failed to parse `Pkg.Revision`: {0}")]
    RevisionInvalid(#[from] RevisionError),
}

#[derive(Debug)]
pub struct Pkg {
    pub revision: Revision,
}

impl Pkg {
    fn from_props(props: &Props) -> Result<Self, PkgError> {
        let revision = props
            .get("Pkg.Revision")
            .ok_or(PkgError::RevisionMissing)
            .and_then(|s| Revision::from_str(s).map_err(Into::into))?;
        Ok(Self { revision })
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to open {path:?}: {source}")]
    OpenFailed {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Failed to parse {path:?}: {source}")]
    ParseFailed {
        path: PathBuf,
        source: java_properties::PropertiesError,
    },
    #[error("Failed to parse `Pkg` in {path:?}: {source}")]
    PkgInvalid { path: PathBuf, source: PkgError },
}

#[derive(Debug)]
pub struct SourceProps {
    pub pkg: Pkg,
}

impl SourceProps {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        let file = std::fs::File::open(path).map_err(|source| Error::OpenFailed {
            path: path.to_owned(),
            source,
        })?;
        let props = java_properties::read(file).map_err(|source| Error::ParseFailed {
            path: path.to_owned(),
            source,
        })?;
        let pkg = Pkg::from_props(&props).map_err(|source| Error::PkgInvalid {
            path: path.to_owned(),
            source,
        })?;
        Ok(Self { pkg })
    }
}
