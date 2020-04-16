use crate::{
    config::app::App,
    util::{self, cli::Report},
};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    path::PathBuf,
};

static DEFAULT_MIN_SDK_VERSION: u32 = 24;
static DEFAULT_VULKAN_VALIDATION: bool = true;
static DEFAULT_PROJECT_DIR: &'static str = "gen/android";
static DEFAULT_NO_DEFAULT_FEATURES: bool = cfg!(feature = "brainium");
static DEFAULT_FEATURES: &'static [&'static str] = {
    #[cfg(feature = "brainium")]
    {
        &["vulkan"]
    }
    #[cfg(not(feature = "brainium"))]
    {
        &[]
    }
};

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

#[derive(Debug)]
pub enum Error {
    ProjectDirInvalid(ProjectDirInvalid),
}

impl Error {
    pub fn report(&self, msg: &str) -> Report {
        match self {
            Self::ProjectDirInvalid(err) => Report::error(
                msg,
                format!("`{}.project-dir` invalid: {}", super::NAME, err),
            ),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Raw {
    min_sdk_version: Option<u32>,
    vulkan_validation: Option<bool>,
    project_dir: Option<String>,
    no_default_features: Option<bool>,
    features: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(skip_serializing)]
    app: App,
    min_sdk_version: u32,
    vulkan_validation: bool,
    project_dir: PathBuf,
    no_default_features: bool,
    features: Vec<String>,
}

impl Config {
    pub fn from_raw(app: App, raw: Option<Raw>) -> Result<Self, Error> {
        let raw = raw.unwrap_or_default();

        let min_sdk_version = raw.min_sdk_version.unwrap_or_else(|| {
            log::info!(
                "`{}.min-sdk-version` not set; defaulting to {}",
                super::NAME,
                DEFAULT_MIN_SDK_VERSION
            );
            DEFAULT_MIN_SDK_VERSION
        });

        let vulkan_validation = raw.vulkan_validation.unwrap_or_else(|| {
            log::info!(
                "`{}.vulkan-validation` not set; defaulting to {}",
                super::NAME,
                DEFAULT_VULKAN_VALIDATION
            );
            DEFAULT_VULKAN_VALIDATION
        });

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
            log::info!(
                "`{}.project-dir` not set; defaulting to {:?}",
                super::NAME,
                DEFAULT_PROJECT_DIR
            );
            Ok(DEFAULT_PROJECT_DIR.into())
        }?;

        let no_default_features = raw.no_default_features.unwrap_or_else(|| {
            log::info!(
                "`{}.no-default-features` not set; defaulting to {:?}",
                super::NAME,
                DEFAULT_NO_DEFAULT_FEATURES
            );
            DEFAULT_NO_DEFAULT_FEATURES
        });

        let features = raw.features.unwrap_or_else(|| {
            log::info!(
                "`{}.features` not set; defaulting to {:?}",
                super::NAME,
                DEFAULT_FEATURES
            );
            DEFAULT_FEATURES.iter().map(|s| s.to_string()).collect()
        });

        Ok(Self {
            app,
            min_sdk_version,
            vulkan_validation,
            project_dir,
            no_default_features,
            features,
        })
    }

    pub fn app(&self) -> &App {
        &self.app
    }

    pub fn min_sdk_version(&self) -> u32 {
        self.min_sdk_version
    }

    pub fn project_dir(&self) -> PathBuf {
        self.app
            .prefix_path(&self.project_dir)
            .join(self.app().name())
    }

    pub fn no_default_features(&self) -> bool {
        self.no_default_features
    }

    pub fn features(&self) -> &[String] {
        &self.features
    }
}
