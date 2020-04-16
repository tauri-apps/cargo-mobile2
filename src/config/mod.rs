pub mod app;
mod raw;

use self::{app::App, raw::*};
#[cfg(feature = "android")]
use crate::android;
#[cfg(feature = "apple")]
use crate::apple;
use crate::{
    opts::Interactivity,
    templating,
    util::{
        cli::{Report, Reportable, TextWrapper},
        submodule::Submodule,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum FromRawError {
    AppConfigInvalid(app::Error),
    #[cfg(feature = "android")]
    AndroidConfigInvalid(android::config::Error),
    #[cfg(feature = "apple")]
    AppleConfigInvalid(apple::config::Error),
}

impl FromRawError {
    pub fn report(&self, msg: &str) -> Report {
        match self {
            Self::AppConfigInvalid(err) => err.report(msg),
            #[cfg(feature = "android")]
            Self::AndroidConfigInvalid(err) => err.report(msg),
            #[cfg(feature = "apple")]
            Self::AppleConfigInvalid(err) => err.report(msg),
        }
    }
}

#[derive(Debug)]
pub enum GenError {
    PromptFailed(PromptError),
    DetectFailed(DetectError),
    CanonicalizeFailed(io::Error),
    FromRawFailed(FromRawError),
    WriteFailed(WriteError),
}

impl Reportable for GenError {
    fn report(&self) -> Report {
        let msg = "Failed to generate config";
        match self {
            Self::PromptFailed(err) => err.report(),
            Self::DetectFailed(err) => err.report(),
            Self::CanonicalizeFailed(err) => {
                Report::error(msg, format!("Failed to canonicalize root dir: {}", err))
            }
            Self::FromRawFailed(err) => err.report(msg),
            Self::WriteFailed(err) => err.report(),
        }
    }
}

#[derive(Debug)]
pub enum LoadOrGenError {
    LoadFailed(LoadError),
    FromRawFailed { path: PathBuf, cause: FromRawError },
    GenFailed(GenError),
}

impl Reportable for LoadOrGenError {
    fn report(&self) -> Report {
        match self {
            Self::LoadFailed(err) => Report::error("Failed to load config", err),
            Self::FromRawFailed { path, cause } => {
                let msg = format!("Config file at {:?} invalid", path);
                cause.report(&msg)
            }
            Self::GenFailed(err) => err.report(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TemplatePack {
    src: PathBuf,
    dest: Option<PathBuf>,
}

impl TemplatePack {
    pub fn with_src(src: impl Into<PathBuf>) -> Self {
        Self {
            src: src.into(),
            dest: None,
        }
    }

    pub fn prefix_src(&self, prefix: &Path, home: &Path) -> PathBuf {
        if let Ok(src) = self.src.strip_prefix("~") {
            home.join(src)
        } else {
            prefix.join(&self.src)
        }
    }

    pub fn prefix_dest(&self, prefix: &Path) -> PathBuf {
        if let Some(dest) = &self.dest {
            prefix.join(dest)
        } else {
            prefix.to_owned()
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    app: App,
    template_packs: Option<Vec<TemplatePack>>,
    submodules: Option<Vec<Submodule>>,
    #[cfg(feature = "android")]
    android: android::config::Config,
    #[cfg(feature = "apple")]
    apple: apple::config::Config,
}

impl Config {
    fn from_raw(root_dir: PathBuf, raw: Raw) -> Result<Self, FromRawError> {
        let app = App::from_raw(root_dir, raw.app).map_err(FromRawError::AppConfigInvalid)?;
        #[cfg(feature = "android")]
        let android = android::config::Config::from_raw(app.clone(), raw.android)
            .map_err(FromRawError::AndroidConfigInvalid)?;
        #[cfg(feature = "apple")]
        let apple = apple::config::Config::from_raw(app.clone(), raw.apple)
            .map_err(FromRawError::AppleConfigInvalid)?;
        Ok(Self {
            app,
            template_packs: raw.template_packs,
            submodules: raw.submodules,
            #[cfg(feature = "android")]
            android,
            #[cfg(feature = "apple")]
            apple,
        })
    }

    fn gen(
        cwd: impl AsRef<Path>,
        interactivity: Interactivity,
        wrapper: &TextWrapper,
    ) -> Result<Self, GenError> {
        let raw = match interactivity {
            Interactivity::Full => Raw::prompt(wrapper).map_err(GenError::PromptFailed),
            Interactivity::None => Raw::detect().map_err(GenError::DetectFailed),
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
        interactivity: Interactivity,
        wrapper: &TextWrapper,
    ) -> Result<Self, LoadOrGenError> {
        let cwd = cwd.as_ref();
        if let Some((root_dir, raw)) = Raw::load(cwd).map_err(LoadOrGenError::LoadFailed)? {
            Self::from_raw(root_dir.clone(), raw).map_err(|cause| LoadOrGenError::FromRawFailed {
                path: root_dir,
                cause,
            })
        } else {
            Self::gen(cwd, interactivity, wrapper).map_err(LoadOrGenError::GenFailed)
        }
    }

    pub fn app(&self) -> &App {
        &self.app
    }

    pub fn template_packs(&self) -> Option<&Vec<TemplatePack>> {
        self.template_packs.as_ref()
    }

    pub fn submodules(&self) -> Option<&Vec<Submodule>> {
        self.submodules.as_ref()
    }

    #[cfg(feature = "android")]
    pub fn android(&self) -> &android::config::Config {
        &self.android
    }

    #[cfg(feature = "apple")]
    pub fn apple(&self) -> &apple::config::Config {
        &self.apple
    }

    pub fn build_a_bike(&self) -> bicycle::Bicycle {
        templating::init(Some(self))
    }
}
