mod raw;

pub use self::raw::*;

use crate::{config::app::App, util};
use serde::Serialize;
use std::{
    fmt::{self, Display},
    path::PathBuf,
};

static DEFAULT_PROJECT_DIR: &'static str = "gen/apple";

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
}

impl Display for ProjectDirInvalid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NormalizationFailed { project_dir, cause } => write!(
                f,
                "Xcode project dir {:?} couldn't be normalized: {}",
                project_dir, cause
            ),
            Self::OutsideOfAppRoot {
                project_dir,
                root_dir,
            } => write!(
                f,
                "Xcode project dir {:?} is outside of the app root dir {:?}",
                project_dir, root_dir,
            ),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    DevelopmentTeamMissing,
    DevelopmentTeamEmpty,
    ProjectDirInvalid(ProjectDirInvalid),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DevelopmentTeamMissing => {
                write!(f, "`{}.development-team` must be specified.", super::NAME)
            }
            Self::DevelopmentTeamEmpty => write!(f, "`{}.development-team` is empty.", super::NAME),
            Self::ProjectDirInvalid(err) => {
                write!(f, "`{}.project-dir` invalid: {}", super::NAME, err)
            }
        }
    }
}

#[serde(rename_all = "kebab-case")]
#[derive(Clone, Debug, Serialize)]
pub struct Config {
    #[serde(skip_serializing)]
    app: App,
    development_team: String,
    project_dir: String,
}

impl Config {
    pub fn from_raw(app: App, raw: Option<Raw>) -> Result<Self, Error> {
        let raw = raw.ok_or_else(|| Error::DevelopmentTeamMissing)?;
        if raw.development_team.is_empty() {
            Err(Error::DevelopmentTeamEmpty)
        } else {
            let project_dir = raw
                .project_dir
                .map(|project_dir| {
                    if project_dir == DEFAULT_PROJECT_DIR {
                        log::warn!("`{}.project-dir` is set to the default value; you can remove it from your config", super::NAME);
                    }
                    if util::normalize_path(&project_dir)
                        .map_err(|cause| Error::ProjectDirInvalid(ProjectDirInvalid::NormalizationFailed {
                            project_dir: project_dir.clone(),
                            cause,
                        }))?
                        .starts_with(app.root_dir())
                    {
                        Ok(project_dir)
                    } else {
                        Err(Error::ProjectDirInvalid(ProjectDirInvalid::OutsideOfAppRoot {
                            project_dir,
                            root_dir: app.root_dir().to_owned(),
                        }))
                    }
                }).unwrap_or_else(|| {
                    log::info!(
                        "`{}.project-dir` not set; defaulting to {}",
                        super::NAME, DEFAULT_PROJECT_DIR
                    );
                    Ok(DEFAULT_PROJECT_DIR.to_owned())
                })?;
            Ok(Self {
                app,
                development_team: raw.development_team,
                project_dir,
            })
        }
    }

    pub fn app(&self) -> &App {
        &self.app
    }

    pub fn project_dir(&self) -> PathBuf {
        self.app.prefix_path(&self.project_dir)
    }

    pub fn workspace_path(&self) -> PathBuf {
        self.project_dir().join(format!(
            "{}.xcodeproj/project.xcworkspace/",
            self.app.name()
        ))
    }

    pub fn export_dir(&self) -> PathBuf {
        self.project_dir().join("build")
    }

    pub fn export_plist_path(&self) -> PathBuf {
        self.project_dir().join("ExportOptions.plist")
    }

    pub fn ipa_path(&self) -> PathBuf {
        self.export_dir().join(format!("{}.ipa", self.scheme()))
    }

    pub fn app_path(&self) -> PathBuf {
        self.export_dir()
            .join(format!("Payload/{}.app", self.app.name()))
    }

    pub fn scheme(&self) -> String {
        format!("{}_iOS", self.app.name())
    }
}
