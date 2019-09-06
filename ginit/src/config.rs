use crate::{
    android::config::{Config as AndroidConfig, RawConfig as AndroidRawConfig},
    app_name,
    ios::config::{Config as IosConfig, RawConfig as IosRawConfig},
};
use heck::SnekCase as _;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

static DEFAULT_APP_ROOT: &'static str = ".";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RawGlobalConfig {
    app_name: String,
    stylized_app_name: Option<String>,
    domain: String,
    app_root: Option<String>,
    // These aren't used anymore, and only kept in so we can emit warnings about them!
    source_root: Option<String>,
    manifest_path: Option<String>,
    asset_path: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
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
                    log::warn!("`global.app_root` is set to the default value; you can remove it from your config");
                }
                app_root
            })
            .unwrap_or_else(|| {
                log::info!(
                    "`global.app_root` not set; defaulting to {}",
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
            LoadError::AppNameInvalid(err) => write!(f, "`global.app_name` invalid: {}", err),
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

/// All paths returned by `Config` methods are prefixed (absolute).
/// Use [`Config::unprefix_path`] if you want to make a path relative to the project root.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    project_root: PathBuf,
    global: GlobalConfig,
    android: AndroidRawConfig,
    ios: IosRawConfig,
}

impl Config {
    fn from_raw(project_root: PathBuf, raw_config: RawConfig) -> Result<Self, app_name::Invalid> {
        Ok(Self {
            project_root,
            global: GlobalConfig::from_raw(raw_config.global)?,
            android: raw_config.android.unwrap_or_default(),
            ios: raw_config.ios,
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
            Ok(Some(
                Self::from_raw(project_root, raw_config).map_err(LoadError::AppNameInvalid)?,
            ))
        } else {
            Ok(None)
        }
    }

    pub fn file_name() -> String {
        format!("{}.toml", crate::NAME)
    }

    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    pub fn prefix_path(&self, path: impl AsRef<Path>) -> PathBuf {
        prefix_path(self.project_root(), path)
    }

    pub fn unprefix_path(&self, path: impl AsRef<Path>) -> Result<PathBuf, UnprefixPathError> {
        unprefix_path(self.project_root(), path)
    }

    pub fn app_name(&self) -> &str {
        &self.global.app_name
    }

    pub fn app_name_snake(&self) -> String {
        self.app_name().to_snek_case()
    }

    pub fn stylized_app_name(&self) -> &str {
        self.global
            .stylized_app_name
            .as_ref()
            .unwrap_or_else(|| &self.global.app_name)
    }

    pub fn reverse_domain(&self) -> String {
        self.global
            .domain
            .clone()
            .split('.')
            .rev()
            .collect::<Vec<_>>()
            .join(".")
    }

    pub fn app_root(&self) -> PathBuf {
        self.prefix_path(&self.global.app_root)
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.app_root().join("Cargo.toml")
    }

    pub fn asset_path(&self) -> PathBuf {
        self.app_root().join("res")
    }

    pub fn android(&self) -> AndroidConfig<'_> {
        AndroidConfig::from_raw(self, &self.android)
    }

    pub fn ios(&self) -> IosConfig<'_> {
        IosConfig::from_raw(self, &self.ios)
    }

    pub(crate) fn insert_template_data(&self, map: &mut bicycle::JsonMap) {
        map.insert("config", &self);
        map.insert("app_name", self.app_name());
        map.insert("app_name_snake", self.app_name_snake());
        map.insert("stylized_app_name", self.stylized_app_name());
        map.insert("reverse_domain", self.reverse_domain());
    }
}
