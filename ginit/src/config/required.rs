use ginit_core::config::RequiredShared;
use serde::Serialize;
use std::{
    collections::HashMap,
    fmt::{self, Debug, Display},
    fs::File,
    io::{self, Write},
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum WriteError {
    SerializationFailed(toml::ser::Error),
    FileCreationFailed { path: PathBuf, cause: io::Error },
    WriteFailed { path: PathBuf, cause: io::Error },
}

impl Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SerializationFailed(err) => write!(f, "Failed to serialize config: {:?}", err),
            Self::FileCreationFailed { path, cause } => {
                write!(f, "Failed to create config file {:?}: {}", path, cause)
            }
            Self::WriteFailed { path, cause } => {
                write!(f, "Failed to write to config file {:?}: {}", path, cause)
            }
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RequiredUmbrella {
    ginit: RequiredShared,
    #[serde(flatten)]
    plugins: HashMap<String, toml::Value>,
}

impl RequiredUmbrella {
    pub fn new(shared: RequiredShared) -> Self {
        Self {
            ginit: shared,
            plugins: Default::default(),
        }
    }

    // pub fn add_plugin<N, R>(&mut self, name: N, required: R) -> Result<(), toml::ser::Error>
    // where
    //     N: Into<String>,
    //     R: RequiredConfigTrait,
    // {
    //     let value = toml::Value::try_from(required)?;
    //     self.plugins.insert(name.into(), value);
    //     Ok(())
    // }

    pub fn add_plugin(&mut self, name: impl Into<String>, value: toml::Value) {
        self.plugins.insert(name.into(), value);
    }

    pub fn write(self, project_root: impl AsRef<Path>) -> Result<(), WriteError> {
        let serialized = toml::to_vec(&self).map_err(WriteError::SerializationFailed)?;
        let path = project_root
            .as_ref()
            .join(format!("{}.toml", ginit_core::NAME));
        let mut file = File::create(&path).map_err(|cause| WriteError::FileCreationFailed {
            path: path.clone(),
            cause,
        })?;
        file.write(&serialized)
            .map_err(|cause| WriteError::WriteFailed {
                path: path.clone(),
                cause,
            })?;
        Ok(())
    }
}
