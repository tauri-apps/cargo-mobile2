pub mod app_name;
mod global;
mod shared;

pub use self::{global::*, shared::*};
use crate::{
    android::config::{Config as AndroidConfig, RawConfig as AndroidRawConfig},
    ios::config::{Config as IosConfig, Error as IosError, RawConfig as IosRawConfig},
};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fs::File,
    io::{self, Read},
    ops::Deref,
    path::{Path, PathBuf},
    rc::Rc,
};

#[derive(Debug)]
pub enum LoadError {
    DiscoverFailed(io::Error),
    OpenFailed(io::Error),
    ReadFailed(io::Error),
    ParseFailed(toml::de::Error),
    GlobalConfigInvalid(ValidationError),
    IosConfigInvalid(IosError),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::DiscoverFailed(err) => write!(
                f,
                "Failed to canonicalize path while searching for project root: {}",
                err
            ),
            LoadError::OpenFailed(err) => write!(f, "Failed to open config file: {}", err),
            LoadError::ReadFailed(err) => write!(f, "Failed to read config file: {}", err),
            LoadError::ParseFailed(err) => write!(f, "Failed to parse config file: {}", err),
            LoadError::GlobalConfigInvalid(err) => write!(f, "`global` config invalid: {}", err),
            LoadError::IosConfigInvalid(err) => write!(f, "`ios` config invalid: {}", err),
        }
    }
}

impl std::error::Error for LoadError {}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RawConfig {
    global: RawGlobalConfig,
    android: Option<AndroidRawConfig>,
    ios: IosRawConfig,
}

/// All paths returned by `Config` methods are prefixed (absolute).
/// Use [`Config::unprefix_path`] if you want to make a path relative to the project root.
#[derive(Clone, Debug, Serialize)]
pub struct Config {
    #[serde(flatten)]
    shared: Rc<SharedConfig>,
    android: AndroidConfig,
    ios: IosConfig,
}

impl Deref for Config {
    type Target = SharedConfig;

    fn deref(&self) -> &Self::Target {
        self.shared()
    }
}

impl Config {
    fn from_raw(project_root: PathBuf, raw_config: RawConfig) -> Result<Self, LoadError> {
        let global = GlobalConfig::from_raw(&project_root, raw_config.global)
            .map_err(LoadError::GlobalConfigInvalid)?;
        let shared = SharedConfig {
            project_root,
            global,
        }
        .into();
        let android =
            AndroidConfig::from_raw(Rc::clone(&shared), raw_config.android.unwrap_or_default());
        let ios = IosConfig::from_raw(Rc::clone(&shared), raw_config.ios)
            .map_err(LoadError::IosConfigInvalid)?;
        Ok(Self {
            shared,
            android,
            ios,
        })
    }

    fn discover_root(cwd: impl AsRef<Path>) -> io::Result<Option<PathBuf>> {
        let file_name = Self::file_name();
        let mut path = cwd.as_ref().canonicalize()?.join(&file_name);
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

    pub fn load(cwd: impl AsRef<Path>) -> Result<Option<Self>, LoadError> {
        if let Some(project_root) = Self::discover_root(cwd).map_err(LoadError::DiscoverFailed)? {
            let path = project_root.join(&Self::file_name());
            let mut file = File::open(&path).map_err(LoadError::OpenFailed)?;
            let mut contents = Vec::new();
            file.read_to_end(&mut contents)
                .map_err(LoadError::ReadFailed)?;
            let raw_config = toml::from_slice(&contents).map_err(LoadError::ParseFailed)?;
            Ok(Some(Self::from_raw(project_root, raw_config)?))
        } else {
            Ok(None)
        }
    }

    pub fn file_name() -> String {
        format!("{}.toml", crate::NAME)
    }

    pub fn shared(&self) -> &Rc<SharedConfig> {
        &self.shared
    }

    pub fn android(&self) -> &AndroidConfig {
        &self.android
    }

    pub fn ios(&self) -> &IosConfig {
        &self.ios
    }

    pub(crate) fn insert_template_data(&self, map: &mut bicycle::JsonMap) {
        map.insert("config", &self);
        map.insert("app-name", self.shared().app_name());
        map.insert("app-name-snake", self.shared().app_name_snake());
        map.insert("stylized-app-name", self.shared().stylized_app_name());
        map.insert("reverse-domain", self.shared().reverse_domain());
    }
}
