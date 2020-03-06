use ginit_core::{
    exports::toml,
    storage::{self, Storage},
};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum LoadError {
    StorageFailed(storage::NoHomeDir),
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
            Self::StorageFailed(err) => write!(f, "Failed to get storage directory: {}", err),
            Self::DeserializeFailed { path, cause } => write!(
                f,
                "Failed to deserialize AIDL cache file at {:?}: {}",
                path, cause
            ),
            Self::ReadFailed { path, cause } => {
                write!(f, "Failed to read AIDL cache file at {:?}: {}", path, cause)
            }
        }
    }
}

#[derive(Debug)]
pub enum SaveError {
    StorageFailed(storage::NoHomeDir),
    SerializeFailed(toml::ser::Error),
    WriteFailed { path: PathBuf, cause: io::Error },
}

impl Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StorageFailed(err) => write!(f, "Failed to get storage directory: {}", err),
            Self::SerializeFailed(err) => write!(f, "Failed to serialize AIDL cache file: {}", err),
            Self::WriteFailed { path, cause } => write!(
                f,
                "Failed to write AIDL cache file at {:?}: {}",
                path, cause
            ),
        }
    }
}

#[derive(Debug)]
pub enum FetchError {}

impl Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

#[derive(Debug)]
pub enum FetchAndSaveError {
    FetchFailed(FetchError),
    SaveFailed(SaveError),
}

impl Display for FetchAndSaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FetchFailed(err) => write!(f, "Failed to fetch AIDL file: {}", err),
            Self::SaveFailed(err) => write!(f, "Failed to save AIDL cache file: {}", err),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    LoadFailed(LoadError),
    FetchFailed(FetchAndSaveError),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LoadFailed(err) => write!(f, "Failed to load AIDL cache file: {}", err),
            Self::FetchFailed(err) => write!(f, "{}", err),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Aidl {
    functions: Vec<String>,
}

impl Aidl {
    fn file_name(version: &str, service: &str) -> String {
        format!("{}-{}.toml", service, version)
    }

    fn path(version: &str, service: &str) -> Result<PathBuf, storage::NoHomeDir> {
        Storage::new().map(|storage| {
            storage
                .plugin_data_dir(crate::NAME)
                .join("aidl-cache")
                .join(Self::file_name(version, service))
        })
    }

    fn load(version: &str, service: &str) -> Result<Option<Self>, LoadError> {
        let path = Self::path(version, service).map_err(LoadError::StorageFailed)?;
        if path.is_file() {
            let bytes = fs::read(&path).map_err(|cause| LoadError::ReadFailed {
                path: path.clone(),
                cause,
            })?;
            toml::from_slice(&bytes)
                .map(Some)
                .map_err(|cause| LoadError::DeserializeFailed { path, cause })
        } else {
            Ok(None)
        }
    }

    fn save(&self, version: &str, service: &str) -> Result<(), SaveError> {
        let path = Self::path(version, service).map_err(SaveError::StorageFailed)?;
        let ser = toml::to_string_pretty(self).map_err(SaveError::SerializeFailed)?;
        fs::write(&path, ser).map_err(|cause| SaveError::WriteFailed { path, cause })
    }

    fn fetch(version: &str, service: &str) -> Result<Self, FetchError> {
        todo!()
    }

    fn fetch_and_save(version: &str, service: &str) -> Result<Self, FetchAndSaveError> {
        let this = Self::fetch(version, service).map_err(FetchAndSaveError::FetchFailed)?;
        this.save(version, service)
            .map_err(FetchAndSaveError::SaveFailed)?;
        Ok(this)
    }

    pub fn load_or_fetch(version: &str, service: &str) -> Result<Self, Error> {
        Self::load(version, service)
            .map_err(Error::LoadFailed)
            .transpose()
            .unwrap_or_else(|| Self::fetch_and_save(version, service).map_err(Error::FetchFailed))
    }

    pub fn index(&self, function: &str) -> Option<usize> {
        self.functions.iter().position(|name| name == function)
    }
}
