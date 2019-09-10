use crate::{
    android::config::{Config as AndroidConfig, RawConfig as AndroidRawConfig},
    app_name,
    ios::config::{Config as IosConfig, Error as IosError, RawConfig as IosRawConfig},
};
use heck::SnekCase as _;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fs::File,
    io::{self, Read},
    ops::Deref,
    path::{Path, PathBuf},
    rc::Rc,
};

static DEFAULT_APP_ROOT: &'static str = ".";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RawGlobalConfig {
    #[serde(alias = "app-name")]
    app_name: String,
    #[serde(alias = "stylized-app-name")]
    stylized_app_name: Option<String>,
    domain: String,
    #[serde(alias = "app-root")]
    app_root: Option<String>,
    // These aren't used anymore, and only kept in so we can emit warnings about them!
    source_root: Option<String>,
    manifest_path: Option<String>,
    asset_path: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct GlobalConfig {
    app_name: String,
    stylized_app_name: Option<String>,
    domain: String,
    app_root: String,
}

impl GlobalConfig {
    fn from_raw(raw_config: RawGlobalConfig) -> Result<Self, app_name::Invalid> {
        if raw_config.source_root.is_some() {
            log::warn!("`global.source_root` specified in {}.toml - this config key is no longer needed, and will be ignored", crate::NAME);
        }
        if raw_config.manifest_path.is_some() {
            log::warn!("`global.manifest_path` specified in {}.toml - this config key is no longer needed, and will be ignored", crate::NAME);
        }
        if raw_config.asset_path.is_some() {
            log::warn!("`global.asset_path` specified in {}.toml - this config key is no longer needed, and will be ignored", crate::NAME);
        }
        Ok(Self {
            app_name: app_name::validate(raw_config.app_name)?,
            stylized_app_name: raw_config.stylized_app_name,
            domain: raw_config.domain,
            app_root: raw_config.app_root.map(|app_root| {
                if app_root.as_str() == DEFAULT_APP_ROOT {
                    log::warn!("`global.app-root` is set to the default value; you can remove it from your config");
                }
                app_root
            })
            .unwrap_or_else(|| {
                log::info!(
                    "`global.app-root` not set; defaulting to {}",
                    DEFAULT_APP_ROOT
                );
                DEFAULT_APP_ROOT.to_owned()
            }),
        })
    }
}

#[derive(Debug)]
pub enum LoadError {
    DiscoverFailed(io::Error),
    OpenFailed(io::Error),
    ReadFailed(io::Error),
    ParseFailed(toml::de::Error),
    AppNameInvalid(app_name::Invalid),
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
            LoadError::AppNameInvalid(err) => write!(f, "`global.app-name` invalid: {}", err),
            LoadError::IosConfigInvalid(err) => write!(f, "iOS config invalid: {}", err),
        }
    }
}

impl std::error::Error for LoadError {}

#[derive(Debug)]
pub enum UnprefixPathError {
    PathNotPrefixed,
}

impl fmt::Display for UnprefixPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnprefixPathError::PathNotPrefixed => write!(
                f,
                "Attempted to remove the project path prefix from a path that wasn't in the project."
            ),
        }
    }
}

pub fn prefix_path(project_root: impl AsRef<Path>, path: impl AsRef<Path>) -> PathBuf {
    project_root.as_ref().join(path)
}

pub fn unprefix_path(
    project_root: impl AsRef<Path>,
    path: impl AsRef<Path>,
) -> Result<PathBuf, UnprefixPathError> {
    path.as_ref()
        .strip_prefix(project_root)
        .map(|path| path.to_owned())
        .map_err(|_| UnprefixPathError::PathNotPrefixed)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RawConfig {
    global: RawGlobalConfig,
    android: Option<AndroidRawConfig>,
    ios: IosRawConfig,
}

#[derive(Clone, Debug, Serialize)]
pub struct SharedConfig {
    project_root: PathBuf,
    global: GlobalConfig,
}

impl SharedConfig {
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    pub fn global(&self) -> &GlobalConfig {
        &self.global
    }

    pub fn prefix_path(&self, path: impl AsRef<Path>) -> PathBuf {
        prefix_path(self.project_root(), path)
    }

    pub fn unprefix_path(&self, path: impl AsRef<Path>) -> Result<PathBuf, UnprefixPathError> {
        unprefix_path(self.project_root(), path)
    }

    pub fn app_name(&self) -> &str {
        &self.global().app_name
    }

    pub fn app_name_snake(&self) -> String {
        self.app_name().to_snek_case()
    }

    pub fn stylized_app_name(&self) -> &str {
        self.global()
            .stylized_app_name
            .as_ref()
            .unwrap_or_else(|| &self.global().app_name)
    }

    pub fn reverse_domain(&self) -> String {
        self.global()
            .domain
            .clone()
            .split('.')
            .rev()
            .collect::<Vec<_>>()
            .join(".")
    }

    pub fn app_root(&self) -> PathBuf {
        self.prefix_path(&self.global().app_root)
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.app_root().join("Cargo.toml")
    }

    pub fn asset_path(&self) -> PathBuf {
        self.app_root().join("res")
    }
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
        let shared = SharedConfig {
            project_root,
            global: GlobalConfig::from_raw(raw_config.global).map_err(LoadError::AppNameInvalid)?,
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
