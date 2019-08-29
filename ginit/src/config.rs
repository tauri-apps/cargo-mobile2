use crate::{
    android::config::{Config as AndroidConfig, RawConfig as AndroidRawConfig},
    ios::config::{Config as IOSConfig, RawConfig as IOSRawConfig},
};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RawConfig {
    global: GlobalConfig,
    android: Option<AndroidRawConfig>,
    ios: IOSRawConfig,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GlobalConfig {
    app_name: String,
    stylized_app_name: Option<String>,
    domain: String,
    source_root: String,
    manifest_path: Option<String>,
    asset_path: String,
}

#[derive(Debug)]
pub enum LoadError {
    DiscoverFailed(io::Error),
    OpenFailed(io::Error),
    ReadFailed(io::Error),
    ParseFailed(toml::de::Error),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::DiscoverFailed(err) => write!(
                f,
                "failed to canonicalize path while searching for project root: {:#?}",
                err
            ),
            LoadError::OpenFailed(err) => write!(f, "failed to open config file: {:#?}", err),
            LoadError::ReadFailed(err) => write!(f, "failed to read config file: {:#?}", err),
            LoadError::ParseFailed(err) => write!(f, "failed to parse config file: {:#?}", err),
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
                "attempted to remove the project path prefix from a path that wasn't in the project"
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

/// All paths returned by `Config` methods are prefixed (absolute).
/// Use [`Config::unprefix_path`] if you want to make a path relative to the project root.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    project_root: PathBuf,
    global: GlobalConfig,
    android: AndroidRawConfig,
    ios: IOSRawConfig,
}

impl Config {
    fn from_raw(project_root: PathBuf, raw_config: RawConfig) -> Self {
        Self {
            project_root,
            global: raw_config.global,
            android: raw_config.android.unwrap_or_default(),
            ios: raw_config.ios,
        }
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
            Ok(Some(Self::from_raw(project_root, raw_config)))
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

    pub fn source_root(&self) -> PathBuf {
        self.prefix_path(&self.global.source_root)
    }

    // TODO: do we actually guarantee this?
    pub fn app_root(&self) -> PathBuf {
        self.source_root().join(self.app_name())
    }

    pub fn manifest_path(&self) -> Option<PathBuf> {
        self.global
            .manifest_path
            .as_ref()
            .map(|path| self.prefix_path(path))
    }

    pub fn asset_path(&self) -> PathBuf {
        self.prefix_path(&self.global.asset_path)
    }

    pub fn android(&self) -> AndroidConfig<'_> {
        AndroidConfig::from_raw(self, &self.android)
    }

    pub fn ios(&self) -> IOSConfig<'_> {
        IOSConfig::from_raw(self, &self.ios)
    }

    pub(crate) fn insert_template_data(&self, map: &mut bicycle::JsonMap) {
        map.insert("config", &self);
        map.insert("app_name", self.app_name());
        map.insert("stylized_app_name", self.stylized_app_name());
        map.insert("reverse_domain", self.reverse_domain());
    }
}
