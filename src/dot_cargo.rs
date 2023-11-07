use crate::{
    config::app::App,
    util::cli::{Report, Reportable},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, io, path::PathBuf};
use thiserror::Error;
use toml::Value;

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("Failed to create \".cargo\" directory at {path}: {cause}")]
    DirCreationFailed { path: PathBuf, cause: io::Error },
    #[error("Failed to rename cargo config from old style {from} to new style {to}: {cause}")]
    MigrateFailed {
        from: PathBuf,
        to: PathBuf,
        cause: io::Error,
    },
    #[error("Failed to read cargo config from {path}: {cause}")]
    ReadFailed { path: PathBuf, cause: io::Error },
    #[error("Failed to deserialize cargo config at {path}: {cause}")]
    DeserializeFailed {
        path: PathBuf,
        cause: toml::de::Error,
    },
}

impl Reportable for LoadError {
    fn report(&self) -> Report {
        Report::error("Failed to load .cargo file", self)
    }
}

#[derive(Debug, Error)]
pub enum WriteError {
    #[error("Failed to serialize cargo config: {0}")]
    SerializeFailed(toml::ser::Error),
    #[error("Failed to create \".cargo\" directory at {path}: {cause}")]
    DirCreationFailed { path: PathBuf, cause: io::Error },
    #[error("Failed to write cargo config to {path}: {cause}")]
    WriteFailed { path: PathBuf, cause: io::Error },
}

impl Reportable for WriteError {
    fn report(&self) -> Report {
        Report::error("Failed to write .cargo", self)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DotCargoBuild {
    target: String,
}

impl DotCargoBuild {
    pub fn new(target: impl Into<String>) -> Self {
        Self {
            target: target.into(),
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct DotCargoTarget {
    pub linker: Option<String>,
    pub rustflags: Vec<String>,
}

impl DotCargoTarget {
    pub fn is_empty(&self) -> bool {
        self.linker.is_none() && self.rustflags.is_empty()
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct DotCargo {
    build: Option<DotCargoBuild>,
    target: BTreeMap<String, DotCargoTarget>,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

impl DotCargo {
    fn create_dir_and_get_path(app: &App) -> Result<PathBuf, (PathBuf, io::Error)> {
        let dir = app.prefix_path(".cargo");
        fs::create_dir_all(&dir)
            .map(|()| dir.join("config.toml"))
            .map_err(|cause| (dir, cause))
    }

    pub fn load(app: &App) -> Result<Self, LoadError> {
        let path = Self::create_dir_and_get_path(app)
            .map_err(|(path, cause)| LoadError::DirCreationFailed { path, cause })?;
        let old_style = path
            .parent()
            .expect("developer error: cargo config path had no parent")
            .join("config");
        if old_style.is_file() {
            // Migrate from old-style cargo config
            std::fs::rename(&old_style, &path).map_err(|cause| LoadError::MigrateFailed {
                from: old_style,
                to: path.clone(),
                cause,
            })?;
        }
        if path.is_file() {
            let toml_str = fs::read_to_string(&path).map_err(|cause| LoadError::ReadFailed {
                path: path.clone(),
                cause,
            })?;
            toml::from_str(&toml_str).map_err(|cause| LoadError::DeserializeFailed { path, cause })
        } else {
            Ok(Self::default())
        }
    }

    pub fn set_default_target(&mut self, target: impl Into<String>) {
        self.build = Some(DotCargoBuild::new(target));
    }

    pub fn insert_target(&mut self, name: impl Into<String>, target: DotCargoTarget) {
        if !target.is_empty() {
            // merging could be nice, but is also very painful...
            self.target.insert(name.into(), target);
        }
    }

    pub fn write(self, app: &App) -> Result<(), WriteError> {
        let path = Self::create_dir_and_get_path(app)
            .map_err(|(path, cause)| WriteError::DirCreationFailed { path, cause })?;
        let ser = toml::to_string_pretty(&self).map_err(WriteError::SerializeFailed)?;
        fs::write(&path, ser).map_err(|cause| WriteError::WriteFailed { path, cause })
    }
}
