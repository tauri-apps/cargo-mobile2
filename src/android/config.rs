use crate::{config::app::App, util};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    path::PathBuf,
};

static DEFAULT_MIN_SDK_VERSION: u32 = 24;
static DEFAULT_PROJECT_DIR: &'static str = "gen/android";

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
                "{:?} contains spaces, which the NDK is remarkably intolerant of.",
                project_dir
            ),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    ProjectDirInvalid(ProjectDirInvalid),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProjectDirInvalid(err) => {
                write!(f, "`{}.project-root` invalid: {}", super::NAME, err)
            }
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Raw {
    min_sdk_version: Option<u32>,
    project_dir: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(skip_serializing)]
    app: App,
    min_sdk_version: u32,
    project_dir: PathBuf,
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

        let project_dir = if let Some(project_dir) = raw.project_dir {
            if project_dir == DEFAULT_PROJECT_DIR {
                log::warn!(
                    "`{}.project-dir` is set to the default value; you can remove it from your config",
                    super::NAME
                );
            }
            let prefixed = app.root_dir().join(&project_dir);
            if util::normalize_path(&prefixed)
                .map_err(|cause| {
                    Error::ProjectDirInvalid(ProjectDirInvalid::NormalizationFailed {
                        project_dir: project_dir.clone(),
                        cause,
                    })
                })?
                .starts_with(app.root_dir())
            {
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

        Ok(Self {
            app,
            min_sdk_version,
            project_dir,
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
}
