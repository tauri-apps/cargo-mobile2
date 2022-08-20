pub mod app;
pub mod metadata;
mod raw;
pub use raw::Raw;

use self::{app::App, raw::*};
#[cfg(target_os = "macos")]
use crate::apple;
use crate::{
    android, bicycle, templating,
    util::cli::{Report, Reportable, TextWrapper},
};
use serde::Serialize;
use std::{
    fmt::Debug,
    io,
    path::{Path, PathBuf},
};
use thiserror::Error;

pub fn file_name() -> String {
    format!("{}.toml", crate::NAME)
}

#[derive(Debug, Error)]
pub enum FromRawError {
    #[error(transparent)]
    AppConfigInvalid(app::Error),
    #[cfg(target_os = "macos")]
    #[error(transparent)]
    AppleConfigInvalid(apple::config::Error),
    #[error(transparent)]
    AndroidConfigInvalid(android::config::Error),
}

impl FromRawError {
    pub fn report(&self, msg: &str) -> Report {
        match self {
            Self::AppConfigInvalid(err) => err.report(msg),
            #[cfg(target_os = "macos")]
            Self::AppleConfigInvalid(err) => err.report(msg),
            Self::AndroidConfigInvalid(err) => err.report(msg),
        }
    }
}

#[derive(Debug, Error)]
pub enum GenError {
    #[error(transparent)]
    PromptFailed(PromptError),
    #[error(transparent)]
    DetectFailed(DetectError),
    #[error("Failed to canonicalize root dir: {0}")]
    CanonicalizeFailed(io::Error),
    #[error(transparent)]
    FromRawFailed(FromRawError),
    #[error(transparent)]
    WriteFailed(WriteError),
}

impl Reportable for GenError {
    fn report(&self) -> Report {
        Report::error("Failed to generate config", self)
    }
}

#[derive(Debug, Error)]
pub enum LoadOrGenError {
    #[error("Failed to load config: {0}")]
    LoadFailed(LoadError),
    #[error("Config file at {path} invalid: {cause}")]
    FromRawFailed { path: PathBuf, cause: FromRawError },
    #[error(transparent)]
    GenFailed(GenError),
}

impl Reportable for LoadOrGenError {
    fn report(&self) -> Report {
        Report::error("Config error", self)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Origin {
    FreshlyMinted,
    Loaded,
}

impl Origin {
    pub fn freshly_minted(self) -> bool {
        matches!(self, Self::FreshlyMinted)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    app: App,
    #[cfg(target_os = "macos")]
    apple: apple::config::Config,
    android: android::config::Config,
}

impl Config {
    pub fn from_raw(root_dir: PathBuf, raw: Raw) -> Result<Self, FromRawError> {
        let app = App::from_raw(root_dir, raw.app).map_err(FromRawError::AppConfigInvalid)?;
        #[cfg(target_os = "macos")]
        let apple = apple::config::Config::from_raw(app.clone(), raw.apple)
            .map_err(FromRawError::AppleConfigInvalid)?;
        let android = android::config::Config::from_raw(app.clone(), raw.android)
            .map_err(FromRawError::AndroidConfigInvalid)?;
        Ok(Self {
            app,
            #[cfg(target_os = "macos")]
            apple,
            android,
        })
    }

    fn gen(
        cwd: impl AsRef<Path>,
        non_interactive: bool,
        wrapper: &TextWrapper,
    ) -> Result<Self, GenError> {
        let raw = if !non_interactive {
            Raw::prompt(wrapper).map_err(GenError::PromptFailed)
        } else {
            Raw::detect(wrapper).map_err(GenError::DetectFailed)
        }?;
        let root_dir = cwd
            .as_ref()
            .canonicalize()
            .map_err(GenError::CanonicalizeFailed)?;
        let config =
            Self::from_raw(root_dir.clone(), raw.clone()).map_err(GenError::FromRawFailed)?;
        log::info!("generated config: {:#?}", config);
        raw.write(&root_dir).map_err(GenError::WriteFailed)?;
        Ok(config)
    }

    pub fn load_or_gen(
        cwd: impl AsRef<Path>,
        non_interactive: bool,
        wrapper: &TextWrapper,
    ) -> Result<(Self, Origin), LoadOrGenError> {
        let cwd = cwd.as_ref();
        if let Some((root_dir, raw)) = Raw::load(cwd).map_err(LoadOrGenError::LoadFailed)? {
            Self::from_raw(root_dir.clone(), raw)
                .map(|config| (config, Origin::Loaded))
                .map_err(|cause| LoadOrGenError::FromRawFailed {
                    path: root_dir,
                    cause,
                })
        } else {
            Self::gen(cwd, non_interactive, wrapper)
                .map(|config| (config, Origin::FreshlyMinted))
                .map_err(LoadOrGenError::GenFailed)
        }
    }

    pub fn path(&self) -> PathBuf {
        self.app().root_dir().join(file_name())
    }

    pub fn app(&self) -> &App {
        &self.app
    }

    #[cfg(target_os = "macos")]
    pub fn apple(&self) -> &apple::config::Config {
        &self.apple
    }

    pub fn android(&self) -> &android::config::Config {
        &self.android
    }

    pub fn build_a_bike(&self) -> bicycle::Bicycle {
        templating::init(Some(self))
    }
}
