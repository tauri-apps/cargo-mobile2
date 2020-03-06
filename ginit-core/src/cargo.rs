use crate::config::shared::Shared;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fmt::{self, Display},
    fs, io,
    path::PathBuf,
};

#[derive(Debug)]
pub enum LoadError {
    DirCreationFailed {
        path: PathBuf,
        cause: io::Error,
    },
    ReadFailed {
        path: PathBuf,
        cause: io::Error,
    },
    DeserializeFailed {
        path: PathBuf,
        cause: toml::de::Error,
    },
}

impl Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DirCreationFailed { path, cause } => write!(
                f,
                "Failed to create \".cargo\" directory at {:?}: {}",
                path, cause
            ),
            Self::ReadFailed { path, cause } => {
                write!(f, "Failed to read cargo config from {:?}: {}", path, cause)
            }
            Self::DeserializeFailed { path, cause } => write!(
                f,
                "Failed to deserialize cargo config at {:?}: {}",
                path, cause
            ),
        }
    }
}

#[derive(Debug)]
pub enum WriteError {
    SerializeFailed(toml::ser::Error),
    DirCreationFailed { path: PathBuf, cause: io::Error },
    WriteFailed { path: PathBuf, cause: io::Error },
}

impl Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SerializeFailed(err) => write!(f, "Failed to serialize cargo config: {}", err),
            Self::DirCreationFailed { path, cause } => write!(
                f,
                "Failed to create \".cargo\" directory at {:?}: {}",
                path, cause
            ),
            Self::WriteFailed { path, cause } => {
                write!(f, "Failed to write cargo config to {:?}: {}", path, cause)
            }
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct DotCargoTarget {
    pub ar: Option<String>,
    pub linker: Option<String>,
    pub rustflags: Vec<String>,
}

impl DotCargoTarget {
    pub fn is_empty(&self) -> bool {
        self.ar.is_none() && self.linker.is_none() && self.rustflags.is_empty()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DotCargo {
    target: BTreeMap<String, DotCargoTarget>,
}

impl DotCargo {
    fn create_dir_and_get_path(shared: &Shared) -> Result<PathBuf, (PathBuf, io::Error)> {
        let dir = shared.prefix_path(".cargo");
        fs::create_dir_all(&dir)
            .map(|()| dir.join("config"))
            .map_err(|cause| (dir, cause))
    }

    pub fn load(shared: &Shared) -> Result<Self, LoadError> {
        let path = Self::create_dir_and_get_path(shared)
            .map_err(|(path, cause)| LoadError::DirCreationFailed { path, cause })?;
        if path.is_file() {
            let bytes = fs::read(&path).map_err(|cause| LoadError::ReadFailed {
                path: path.clone(),
                cause,
            })?;
            toml::from_slice(&bytes).map_err(|cause| LoadError::DeserializeFailed { path, cause })
        } else {
            Ok(Self {
                target: Default::default(),
            })
        }
    }

    pub fn insert_target(&mut self, name: impl Into<String>, target: DotCargoTarget) {
        if !target.is_empty() {
            // merging could be nice, but is also very painful...
            self.target.insert(name.into(), target);
        }
    }

    pub fn write(&self, shared: &Shared) -> Result<(), WriteError> {
        let path = Self::create_dir_and_get_path(shared)
            .map_err(|(path, cause)| WriteError::DirCreationFailed { path, cause })?;
        let ser = toml::to_string_pretty(self).map_err(WriteError::SerializeFailed)?;
        fs::write(&path, ser).map_err(|cause| WriteError::WriteFailed { path, cause })
    }
}
