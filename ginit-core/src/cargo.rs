use crate::config::shared::Shared;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fmt::{self, Display},
    fs::{self, File},
    io::{self, Read, Write},
    path::PathBuf,
};

#[derive(Debug)]
pub enum LoadError {
    DirCreationFailed(io::Error),
    OpenFailed(io::Error),
    ReadFailed(io::Error),
    DeserializationFailed(toml::de::Error),
}

impl Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DirCreationFailed(err) => {
                write!(f, "Failed to create \".cargo\" directory: {}", err)
            }
            Self::OpenFailed(err) => {
                write!(f, "Failed to open \".cargo/config\" for reading: {}", err)
            }
            Self::ReadFailed(err) => {
                write!(f, "Failed to read config from \".cargo/config\": {}", err)
            }
            Self::DeserializationFailed(err) => {
                write!(f, "Failed to deserialize cargo config: {}", err)
            }
        }
    }
}

#[derive(Debug)]
pub enum WriteError {
    SerializationFailed(toml::ser::Error),
    DirCreationFailed(io::Error),
    CreateFailed(io::Error),
    WriteFailed(io::Error),
}

impl Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SerializationFailed(err) => {
                write!(f, "Failed to serialize cargo config: {}", err)
            }
            Self::DirCreationFailed(err) => {
                write!(f, "Failed to create \".cargo\" directory: {}", err)
            }
            Self::CreateFailed(err) => {
                write!(f, "Failed to open \".cargo/config\" for writing: {}", err)
            }
            Self::WriteFailed(err) => {
                write!(f, "Failed to write config to \".cargo/config\": {}", err)
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
    fn create_dir_and_get_path(shared: &Shared) -> io::Result<PathBuf> {
        let dir = shared.prefix_path(".cargo");
        fs::create_dir_all(&dir).map(|()| dir.join("config"))
    }

    pub fn load(shared: &Shared) -> Result<Self, LoadError> {
        let path = Self::create_dir_and_get_path(shared).map_err(LoadError::DirCreationFailed)?;
        if path.is_file() {
            let mut file = File::open(path).map_err(LoadError::OpenFailed)?;
            let mut raw = Vec::new();
            file.read_to_end(&mut raw).map_err(LoadError::ReadFailed)?;
            toml::from_slice(&raw).map_err(LoadError::DeserializationFailed)
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
        let serialized = toml::to_string_pretty(self).map_err(WriteError::SerializationFailed)?;
        let path = Self::create_dir_and_get_path(shared).map_err(WriteError::DirCreationFailed)?;
        let mut file = File::create(path).map_err(WriteError::CreateFailed)?;
        file.write_all(serialized.as_bytes())
            .map_err(WriteError::WriteFailed)
    }
}
