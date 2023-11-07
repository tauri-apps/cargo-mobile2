use super::app;
#[cfg(target_os = "macos")]
use crate::apple;
use crate::{
    android,
    util::cli::{Report, Reportable, TextWrapper},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug, Error)]
pub enum PromptError {
    #[error("Failed to prompt for `app` config: {0}")]
    AppFailed(app::PromptError),
    #[cfg(target_os = "macos")]
    #[error("Failed to prompt for `app` config: {0}")]
    AppleFailed(apple::config::PromptError),
}

impl Reportable for PromptError {
    fn report(&self) -> Report {
        Report::error("Prompt error", self)
    }
}

#[derive(Debug, Error)]
pub enum DetectError {
    #[error("Failed to detect `app` config: {0}")]
    AppFailed(app::DetectError),
    #[cfg(target_os = "macos")]
    #[error("Failed to detect `app` config: {0}")]
    AppleFailed(apple::config::DetectError),
}

impl Reportable for DetectError {
    fn report(&self) -> Report {
        Report::error("Detection error", self)
    }
}

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("Failed to canonicalize path while searching for config file: {0}")]
    Discover(io::Error),
    #[error("Failed to read config file at {path}: {cause}")]
    Read { path: PathBuf, cause: io::Error },
    #[error("Failed to parse config file at {path}: {cause}")]
    Parse {
        path: PathBuf,
        cause: toml::de::Error,
    },
}

#[derive(Debug, Error)]
pub enum WriteError {
    #[error("Failed to serialize config: {0}")]
    Serialize(toml::ser::Error),
    #[error("Failed to write config: {0}")]
    Write(io::Error),
}

impl Reportable for WriteError {
    fn report(&self) -> Report {
        Report::error("Failed to write config", self)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Raw {
    pub app: app::Raw,
    #[cfg(target_os = "macos")]
    pub apple: Option<apple::config::Raw>,
    pub android: Option<android::config::Raw>,
}

impl Raw {
    pub fn prompt(wrapper: &TextWrapper) -> Result<Self, PromptError> {
        let app = app::Raw::prompt(wrapper).map_err(PromptError::AppFailed)?;
        #[cfg(target_os = "macos")]
        let apple = apple::config::Raw::prompt(wrapper).map_err(PromptError::AppleFailed)?;
        Ok(Self {
            app,
            #[cfg(target_os = "macos")]
            apple: Some(apple),
            android: None,
        })
    }

    pub fn detect(wrapper: &TextWrapper) -> Result<Self, DetectError> {
        let app = app::Raw::detect(wrapper).map_err(DetectError::AppFailed)?;
        #[cfg(target_os = "macos")]
        let apple = apple::config::Raw::detect().map_err(DetectError::AppleFailed)?;
        Ok(Self {
            app,
            #[cfg(target_os = "macos")]
            apple: Some(apple),
            android: None,
        })
    }

    pub fn discover_root(cwd: impl AsRef<Path>) -> io::Result<Option<PathBuf>> {
        let file_name = super::file_name();
        let mut path = cwd.as_ref().canonicalize()?.join(&file_name);
        log::info!("looking for config file at {:?}", path);
        while !path.exists() {
            if let Some(parent) = path.parent().and_then(Path::parent) {
                path = parent.join(&file_name);
                log::info!("looking for config file at {:?}", path);
            } else {
                log::info!("no config file was ever found");
                return Ok(None);
            }
        }
        log::info!("found config file at {:?}", path);
        path.pop();
        Ok(Some(path))
    }

    pub fn load(cwd: impl AsRef<Path>) -> Result<Option<(PathBuf, Self)>, LoadError> {
        Self::discover_root(cwd)
            .map_err(LoadError::Discover)?
            .map(|root_dir| {
                let path = root_dir.join(super::file_name());
                let toml_str = fs::read_to_string(&path).map_err(|cause| LoadError::Read {
                    path: path.clone(),
                    cause,
                })?;
                toml::from_str::<Self>(&toml_str)
                    .map(|raw| (root_dir, raw))
                    .map_err(|cause| LoadError::Parse {
                        path: path.clone(),
                        cause,
                    })
            })
            .transpose()
    }

    pub fn write(&self, root_dir: &Path) -> Result<(), WriteError> {
        let toml_str = toml::to_string(self).map_err(WriteError::Serialize)?;
        let path = root_dir.join(super::file_name());
        log::info!("writing config to {:?}", path);
        fs::write(path, toml_str).map_err(WriteError::Write)
    }
}
