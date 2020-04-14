pub mod app;
mod raw;

use self::{app::App, raw::*};
use crate::{
    android, apple,
    opts::Interactivity,
    templating,
    util::{cli::TextWrapper, submodule::Submodule},
};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Debug, Display},
    io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum FromRawError {
    AppConfigInvalid(app::Error),
    AndroidConfigInvalid(android::config::Error),
    AppleConfigInvalid(apple::config::Error),
}

impl Display for FromRawError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AppConfigInvalid(err) => write!(f, "`{}` config invalid: {}", app::KEY, err),
            Self::AndroidConfigInvalid(err) => {
                write!(f, "`{}` config invalid: {}", android::NAME, err)
            }
            Self::AppleConfigInvalid(err) => write!(f, "`{}` config invalid: {}", apple::NAME, err),
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

impl Display for GenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PromptFailed(err) => write!(f, "Failed to prompt for config: {}", err),
            Self::DetectFailed(err) => write!(f, "Failed to detect config: {}", err),
            Self::CanonicalizeFailed(err) => write!(f, "Failed to canonicalize root dir: {}", err),
            Self::FromRawFailed(err) => write!(f, "Generated config invalid: {}", err),
            Self::WriteFailed(err) => write!(f, "{}", err),
        }
    }
}

#[derive(Debug)]
pub enum LoadOrGenError {
    LoadFailed(LoadError),
    FromRawFailed { path: PathBuf, cause: FromRawError },
    GenFailed(GenError),
}

impl Display for LoadOrGenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LoadFailed(err) => write!(f, "Failed to load config: {}", err),
            Self::FromRawFailed { path, cause } => {
                write!(f, "Config file at {:?} invalid: {}", path, cause)
            }
            Self::GenFailed(err) => write!(f, "Failed to generate config: {}", err),
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
    android: android::config::Config,
    apple: apple::config::Config,
}

impl Config {
    fn from_raw(
        root_dir: PathBuf,
        Raw {
            app,
            template_packs,
            submodules,
            android,
            apple,
        }: Raw,
    ) -> Result<Self, FromRawError> {
        let app = App::from_raw(root_dir, app).map_err(FromRawError::AppConfigInvalid)?;
        let android = android::config::Config::from_raw(app.clone(), android)
            .map_err(FromRawError::AndroidConfigInvalid)?;
        let apple = apple::config::Config::from_raw(app.clone(), apple)
            .map_err(FromRawError::AppleConfigInvalid)?;
        Ok(Self {
            app,
            template_packs,
            submodules,
            android,
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

    pub fn android(&self) -> &android::config::Config {
        &self.android
    }

    pub fn apple(&self) -> &apple::config::Config {
        &self.apple
    }

    pub fn build_a_bike(&self) -> bicycle::Bicycle {
        templating::init(Some(self))
    }
}
