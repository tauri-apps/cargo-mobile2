use ginit_core::{
    config::{Config as CoreConfig, ConfigTrait},
    util,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt, path::PathBuf};

const DEFAULT_MIN_SDK_VERSION: u32 = 24;
static DEFAULT_PROJECT_ROOT: &'static str = "gen/android";

#[derive(Debug)]
pub enum ProjectRootInvalid {
    NormalizationFailed {
        android_project_root: String,
        cause: util::NormalizationError,
    },
    OutsideOfProject {
        android_project_root: String,
        project_root: PathBuf,
    },
}

impl fmt::Display for ProjectRootInvalid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NormalizationFailed {
                android_project_root,
                cause,
            } => write!(
                f,
                "{:?} couldn't be normalized: {}",
                android_project_root, cause
            ),
            Self::OutsideOfProject {
                android_project_root,
                project_root,
            } => write!(
                f,
                "{:?} is outside of the project root ({:?}).",
                android_project_root, project_root,
            ),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    ProjectRootInvalid(ProjectRootInvalid),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProjectRootInvalid(err) => write!(f, "`android.project-root` invalid: {}", err),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Raw {
    #[serde(alias = "min-sdk-version")]
    min_sdk_version: Option<u32>,
    #[serde(alias = "project-root")]
    project_root: Option<String>,
    targets: Option<HashMap<String, HashMap<String, String>>>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(skip_serializing)]
    shared: CoreConfig,
    min_sdk_version: u32,
    project_root: String,
}

impl ConfigTrait for Config {
    type Raw = Raw;
    type Error = Error;

    fn from_raw(shared: CoreConfig, raw: Option<Self::Raw>) -> Result<Self, Self::Error> {
        let raw = raw.unwrap_or_default();
        if raw.targets.is_some() {
            log::warn!("`android.targets` specified in {}.toml - this config key is no longer needed, and will be ignored", ginit_core::NAME);
        }
        let min_sdk_version = raw
            .min_sdk_version
            .map(|min_sdk_version| {
                if min_sdk_version == DEFAULT_MIN_SDK_VERSION {
                    log::warn!("`android.min-sdk-version` is set to the default value; you can remove it from your config");
                }
                min_sdk_version
            })
            .unwrap_or_else(|| {
                log::info!(
                    "`android.min-sdk-version` not set; defaulting to {}",
                    DEFAULT_MIN_SDK_VERSION
                );
                DEFAULT_MIN_SDK_VERSION
            });
        let project_root = raw
            .project_root
            .map(|project_root| {
                if project_root == DEFAULT_PROJECT_ROOT {
                    log::warn!("`android.project-root` is set to the default value; you can remove it from your config");
                }
                let prefixed = shared.project_root().join(&project_root);
                if util::normalize_path(&prefixed)
                    .map_err(|cause| Error::ProjectRootInvalid(ProjectRootInvalid::NormalizationFailed {
                        android_project_root: project_root.clone(),
                        cause,
                    }))?
                    .starts_with(shared.project_root())
                {
                    Ok(project_root)
                } else {
                    Err(Error::ProjectRootInvalid(ProjectRootInvalid::OutsideOfProject {
                        android_project_root: project_root,
                        project_root: shared.project_root().to_owned(),
                    }))
                }
            })
            .unwrap_or_else(|| {
                log::info!(
                    "`android.project-root` not set; defaulting to {}",
                    DEFAULT_PROJECT_ROOT
                );
                Ok(DEFAULT_PROJECT_ROOT.to_owned())
            })?;
        Ok(Self {
            shared,
            min_sdk_version,
            project_root,
        })
    }

    fn shared(&self) -> &CoreConfig {
        &self.shared
    }
}

impl Config {
    pub fn min_sdk_version(&self) -> u32 {
        self.min_sdk_version
    }

    pub fn project_path(&self) -> PathBuf {
        self.shared
            .project_root()
            .join(&self.project_root)
            .join(self.shared.app_name())
    }
}
