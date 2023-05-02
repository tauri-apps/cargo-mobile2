use crate::{
    config::app::App,
    util::{self, cli::Report},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::{self, Display},
    path::PathBuf,
};
use thiserror::Error;

const DEFAULT_MIN_SDK_VERSION: u32 = 24;
pub const DEFAULT_VULKAN_VALIDATION: bool = true;
static DEFAULT_PROJECT_DIR: &str = "gen/android";

const fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct AssetPackInfo {
    pub name: String,
    pub delivery_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Metadata {
    #[serde(default = "default_true")]
    pub supported: bool,
    #[serde(default)]
    pub no_default_features: bool,
    pub cargo_args: Option<Vec<String>>,
    pub features: Option<Vec<String>>,
    pub app_sources: Option<Vec<String>>,
    pub app_plugins: Option<Vec<String>>,
    pub project_dependencies: Option<Vec<String>>,
    pub app_dependencies: Option<Vec<String>>,
    pub app_dependencies_platform: Option<Vec<String>>,
    pub asset_packs: Option<Vec<AssetPackInfo>>,
    pub app_activity_name: Option<String>,
    pub app_permissions: Option<Vec<String>>,
    pub app_theme_parent: Option<String>,
    pub env_vars: Option<HashMap<String, String>>,
    pub vulkan_validation: Option<bool>,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            supported: true,
            no_default_features: false,
            cargo_args: None,
            features: None,
            app_sources: None,
            app_plugins: None,
            project_dependencies: None,
            app_dependencies: None,
            app_dependencies_platform: None,
            asset_packs: None,
            app_activity_name: None,
            app_permissions: None,
            app_theme_parent: None,
            env_vars: None,
            vulkan_validation: None,
        }
    }
}

impl Metadata {
    pub const fn supported(&self) -> bool {
        self.supported
    }

    pub fn no_default_features(&self) -> bool {
        self.no_default_features
    }

    pub fn cargo_args(&self) -> Option<&[String]> {
        self.cargo_args.as_deref()
    }

    pub fn features(&self) -> Option<&[String]> {
        self.features.as_deref()
    }

    pub fn app_sources(&self) -> &[String] {
        self.app_sources.as_deref().unwrap_or(&[])
    }

    pub fn app_plugins(&self) -> Option<&[String]> {
        self.app_plugins.as_deref()
    }

    pub fn project_dependencies(&self) -> Option<&[String]> {
        self.project_dependencies.as_deref()
    }

    pub fn app_dependencies(&self) -> Option<&[String]> {
        self.app_dependencies.as_deref()
    }

    pub fn app_dependencies_platform(&self) -> Option<&[String]> {
        self.app_dependencies_platform.as_deref()
    }

    pub fn asset_packs(&self) -> Option<&[AssetPackInfo]> {
        self.asset_packs.as_deref()
    }

    pub fn app_activity_name(&self) -> Option<&str> {
        self.app_activity_name.as_deref()
    }

    pub fn app_permissions(&self) -> Option<&[String]> {
        self.app_permissions.as_deref()
    }

    pub fn app_theme_parent(&self) -> Option<&str> {
        self.app_theme_parent.as_deref()
    }

    pub fn vulkan_validation(&self) -> Option<bool> {
        self.vulkan_validation
    }
}

#[derive(Debug)]
pub enum ProjectDirInvalid {
    NormalizationFailed {
        project_dir: String,
        cause: util::NormalizationError,
    },
    OutsideOfAppRoot {
        project_dir: String,
        root_dir: PathBuf,
    },
    ContainsSpaces {
        project_dir: String,
    },
}

impl Display for ProjectDirInvalid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NormalizationFailed { project_dir, cause } => {
                write!(f, "{:?} couldn't be normalized: {}", project_dir, cause)
            }
            Self::OutsideOfAppRoot {
                project_dir,
                root_dir,
            } => write!(
                f,
                "{:?} is outside of the app root {:?}",
                project_dir, root_dir,
            ),
            Self::ContainsSpaces { project_dir } => write!(
                f,
                "{:?} contains spaces, which the NDK is remarkably intolerant of",
                project_dir
            ),
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("android.project-dir invalid: {0}")]
    ProjectDirInvalid(ProjectDirInvalid),
}

impl Error {
    pub fn report(&self, msg: &str) -> Report {
        Report::error(msg, self)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Raw {
    pub min_sdk_version: Option<u32>,
    pub project_dir: Option<String>,
    pub no_default_features: Option<bool>,
    pub features: Option<Vec<String>>,
    #[serde(default)]
    pub logcat_filter_specs: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(skip_serializing)]
    app: App,
    min_sdk_version: u32,
    project_dir: PathBuf,
    logcat_filter_specs: Vec<String>,
}

impl Config {
    pub fn from_raw(app: App, raw: Option<Raw>) -> Result<Self, Error> {
        let raw = raw.unwrap_or_default();

        let min_sdk_version = raw.min_sdk_version.unwrap_or(DEFAULT_MIN_SDK_VERSION);

        let project_dir = if let Some(project_dir) = raw.project_dir {
            if project_dir == DEFAULT_PROJECT_DIR {
                log::warn!(
                    "`{}.project-dir` is set to the default value; you can remove it from your config",
                    super::NAME
                );
            }
            if util::under_root(&project_dir, app.root_dir()).map_err(|cause| {
                Error::ProjectDirInvalid(ProjectDirInvalid::NormalizationFailed {
                    project_dir: project_dir.clone(),
                    cause,
                })
            })? {
                if !project_dir.contains(' ') {
                    Ok(project_dir.into())
                } else {
                    Err(Error::ProjectDirInvalid(
                        ProjectDirInvalid::ContainsSpaces { project_dir },
                    ))
                }
            } else {
                Err(Error::ProjectDirInvalid(
                    ProjectDirInvalid::OutsideOfAppRoot {
                        project_dir,
                        root_dir: app.root_dir().to_owned(),
                    },
                ))
            }
        } else {
            Ok(DEFAULT_PROJECT_DIR.into())
        }?;

        Ok(Self {
            app,
            min_sdk_version,
            project_dir,
            logcat_filter_specs: raw.logcat_filter_specs,
        })
    }

    pub fn app(&self) -> &App {
        &self.app
    }

    pub fn logcat_filter_specs(&self) -> &[String] {
        &self.logcat_filter_specs
    }

    pub fn so_name(&self) -> String {
        format!("lib{}.so", self.app().lib_name())
    }

    pub fn min_sdk_version(&self) -> u32 {
        self.min_sdk_version
    }

    pub fn project_dir(&self) -> PathBuf {
        self.app.prefix_path(&self.project_dir)
    }

    pub fn project_dir_exists(&self) -> bool {
        self.project_dir().is_dir()
    }
}
