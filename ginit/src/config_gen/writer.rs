use ginit_core::{
    config::{gen, shared, umbrella::Umbrella},
    exports::toml,
    opts,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::{self, Debug, Display},
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum LoadError {
    ReadFailed { path: PathBuf, cause: io::Error },
    DeserializationFailed(toml::de::Error),
}

impl Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadFailed { path, cause } => {
                write!(f, "Failed to read config file {:?}: {}", path, cause)
            }
            Self::DeserializationFailed(err) => {
                write!(f, "Failed to deserialize config: {:?}", err)
            }
        }
    }
}

#[derive(Debug)]
pub enum LinkError {
    DirectoryReadFailed {
        path: PathBuf,
        cause: io::Error,
    },
    DirectoryEntryFailed {
        path: PathBuf,
        cause: io::Error,
    },
    FileNameMalformed {
        path: PathBuf,
    },
    FileStemInvalidUtf8 {
        path: PathBuf,
    },
    FileReadFailed {
        path: PathBuf,
        cause: io::Error,
    },
    DeserializationFailed {
        path: PathBuf,
        cause: toml::de::Error,
    },
    FileDeletionFailed {
        path: PathBuf,
        cause: io::Error,
    },
}

impl Display for LinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DirectoryReadFailed { path, cause } => write!(
                f,
                "Failed to list contents of temp config directory {:?}: {}",
                path, cause
            ),
            Self::DirectoryEntryFailed { path, cause } => write!(
                f,
                "Failed to get path of entry in temp config directory {:?}: {}",
                path, cause
            ),
            Self::FileNameMalformed { path } => {
                write!(f, "Temp config file path has no stem: {:?}", path)
            }
            Self::FileStemInvalidUtf8 { path } => write!(
                f,
                "Temp config file stem contained invalid UTF-8: {:?}",
                path
            ),
            Self::FileReadFailed { path, cause } => {
                write!(f, "Failed to read temp config file {:?}: {}", path, cause)
            }
            Self::DeserializationFailed { path, cause } => write!(
                f,
                "Failed to deserialize contents of temp config file {:?}: {}",
                path, cause
            ),
            Self::FileDeletionFailed { path, cause } => {
                write!(f, "Failed to delete temp config file {:?}: {}", path, cause)
            }
        }
    }
}

#[derive(Debug)]
pub enum WriteError {
    SerializationFailed(toml::ser::Error),
    WriteFailed { path: PathBuf, cause: io::Error },
}

impl Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SerializationFailed(err) => write!(f, "Failed to serialize config: {:?}", err),
            Self::WriteFailed { path, cause } => {
                write!(f, "Failed to write config file {:?}: {}", path, cause)
            }
        }
    }
}

#[derive(Debug)]
pub enum LinkAndWriteError {
    LinkFailed(LinkError),
    WriteFailed(WriteError),
}

impl Display for LinkAndWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LinkFailed(err) => write!(f, "{}", err),
            Self::WriteFailed(err) => write!(f, "{}", err),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Writer {
    #[serde(rename = "ginit")]
    shared: shared::Raw,
    #[serde(flatten)]
    plugins: HashMap<String, toml::Value>,
}

impl Writer {
    fn path(project_root: impl AsRef<Path>) -> PathBuf {
        project_root.as_ref().join(Umbrella::file_name())
    }

    pub fn load_existing(
        clobbering: opts::Clobbering,
        project_root: impl AsRef<Path>,
        shared: shared::Raw,
    ) -> Result<Self, LoadError> {
        let path = Self::path(project_root);
        // To avoid erasing sections, we first load in the existing config.
        let plugins = {
            let plugins = if path.is_file() {
                let bytes = fs::read(&path).map_err(|cause| LoadError::ReadFailed {
                    path: path.clone(),
                    cause,
                })?;
                Some(
                    toml::from_slice::<Self>(&bytes)
                        .map_err(LoadError::DeserializationFailed)
                        .map(|Self { plugins, .. }| plugins),
                )
            } else {
                None
            }
            .transpose();
            match clobbering {
                opts::Clobbering::Forbid => plugins?,
                // If clobbering is allowed, then we proceed even if we failed
                // to read the existing config.
                opts::Clobbering::Allow => plugins.unwrap_or_default(),
            }
        }
        .unwrap_or_default();
        Ok(Self { shared, plugins })
    }

    fn add_plugin(&mut self, key: impl Into<String>, section: toml::Value) {
        self.plugins.insert(key.into(), section);
    }

    fn link(&mut self) -> Result<(), LinkError> {
        let dir = gen::temp_subdir();
        for entry in fs::read_dir(&dir).map_err(|cause| LinkError::DirectoryReadFailed {
            path: dir.clone(),
            cause,
        })? {
            let path = entry
                .map_err(|cause| LinkError::DirectoryEntryFailed {
                    path: dir.clone(),
                    cause,
                })?
                .path();
            let name = path
                .file_stem()
                .ok_or_else(|| LinkError::FileNameMalformed { path: path.clone() })?
                .to_str()
                .ok_or_else(|| LinkError::FileStemInvalidUtf8 { path: path.clone() })?;
            let bytes = fs::read(&path).map_err(|cause| LinkError::FileReadFailed {
                path: path.clone(),
                cause,
            })?;
            if !bytes.is_empty() {
                let de =
                    toml::from_slice(&bytes).map_err(|cause| LinkError::DeserializationFailed {
                        path: path.clone(),
                        cause,
                    })?;
                self.add_plugin(name, de);
            }
            log::info!("removing temp config file at {:?}", path);
            fs::remove_file(&path).map_err(|cause| LinkError::FileDeletionFailed {
                path: path.clone(),
                cause,
            })?;
        }
        Ok(())
    }

    fn write(self, project_root: impl AsRef<Path>) -> Result<(), WriteError> {
        let serialized = toml::to_vec(&self).map_err(WriteError::SerializationFailed)?;
        let path = Self::path(project_root);
        fs::write(&path, serialized).map_err(|cause| WriteError::WriteFailed { path, cause })
    }

    pub fn link_and_write(
        mut self,
        project_root: impl AsRef<Path>,
    ) -> Result<(), LinkAndWriteError> {
        self.link().map_err(LinkAndWriteError::LinkFailed)?;
        self.write(project_root)
            .map_err(LinkAndWriteError::WriteFailed)
    }
}
