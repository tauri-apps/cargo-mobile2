use crate::util::{
    self,
    submodule::{self, Submodule},
    Git,
};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum RemotePackParseError {
    ReadFailed {
        path: PathBuf,
        cause: io::Error,
    },
    ParseFailed {
        path: PathBuf,
        cause: toml::de::Error,
    },
    NoHomeDir(util::NoHomeDir),
}

impl Display for RemotePackParseError {
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
        }
    }
}

#[derive(Debug)]
pub enum RemotePackResolveError {
    SubmoduleFailed(submodule::Error),
    PackNotFound(PathBuf),
}

impl Display for RemotePackResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SubmoduleFailed(err) => write!(f, "Failed to initialize submodule: {}", err),
            Self::PackNotFound(path) => write!(f, "Template pack wasn't found at {:?}", path),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RemotePack {
    path: PathBuf,
    submodule: Option<Submodule>,
}

impl RemotePack {
    pub fn parse(path: impl AsRef<Path>) -> Result<Self, RemotePackParseError> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|cause| RemotePackParseError::ReadFailed {
            path: path.to_owned(),
            cause,
        })?;
        let mut this = toml::from_slice::<Self>(&bytes).map_err(|cause| {
            RemotePackParseError::ParseFailed {
                path: path.to_owned(),
                cause,
            }
        })?;
        this.path = util::expand_home(&this.path).map_err(RemotePackParseError::NoHomeDir)?;
        Ok(this)
    }

    pub fn submodule_path(&self) -> Option<&Path> {
        self.submodule.as_ref().map(|submodule| submodule.path())
    }

    pub fn resolve(&self, git: Git<'_>) -> Result<&Path, RemotePackResolveError> {
        if let Some(submodule) = &self.submodule {
            submodule
                .init(git)
                .map_err(RemotePackResolveError::SubmoduleFailed)?;
        }
        if self.path.exists() {
            Ok(&self.path)
        } else {
            Err(RemotePackResolveError::PackNotFound(self.path.clone()))
        }
    }
}
