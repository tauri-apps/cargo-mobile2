pub mod raw;

use self::raw::Raw;
use ginit_core::{
    config::{shared::Shared, ConfigTrait},
    util,
};
use serde::Serialize;
use std::{
    fmt::{self, Display},
    path::PathBuf,
};

static DEFAULT_PROJECT_PATH: &'static str = "gen/ios";

#[derive(Debug)]
pub enum ProjectPathInvalid {
    NormalizationFailed {
        ios_project_path: String,
        cause: util::NormalizationError,
    },
    OutsideOfProject {
        ios_project_path: String,
        project_path: PathBuf,
    },
}

impl Display for ProjectPathInvalid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NormalizationFailed {
                ios_project_path,
                cause,
            } => write!(
                f,
                "{:?} couldn't be normalized: {}",
                ios_project_path, cause
            ),
            Self::OutsideOfProject {
                ios_project_path,
                project_path,
            } => write!(
                f,
                "{:?} is outside of the project root ({:?}).",
                ios_project_path, project_path,
            ),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    DevelopmentTeamMissing,
    DevelopmentTeamEmpty,
    ProjectPathInvalid(ProjectPathInvalid),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DevelopmentTeamMissing => write!(f, "`ios.development-team` must be specified."),
            Self::DevelopmentTeamEmpty => write!(f, "`ios.development-team` is empty."),
            Self::ProjectPathInvalid(err) => write!(f, "`ios.project-path` invalid: {}", err),
        }
    }
}

#[serde(rename_all = "kebab-case")]
#[derive(Clone, Debug, Serialize)]
pub struct Config {
    #[serde(skip_serializing)]
    shared: Shared,
    development_team: String,
    project_path: String,
}

impl ConfigTrait for Config {
    type Raw = Raw;
    type Error = Error;
    fn from_raw(shared: Shared, raw: Option<Self::Raw>) -> Result<Self, Self::Error> {
        let raw = raw.ok_or_else(|| Error::DevelopmentTeamMissing)?;
        if raw.targets.is_some() {
            log::warn!("`ios.targets` specified in {}.toml - this config key is no longer necessary, and is ignored", ginit_core::NAME);
        }
        if raw.development_team.is_empty() {
            Err(Error::DevelopmentTeamEmpty)
        } else {
            let project_path = raw
                .project_path
                .map(|project_path| {
                    if project_path == DEFAULT_PROJECT_PATH {
                        log::warn!("`ios.project-path` is set to the default value; you can remove it from your config");
                    }
                    if util::normalize_path(&project_path)
                        .map_err(|cause| Error::ProjectPathInvalid(ProjectPathInvalid::NormalizationFailed {
                            ios_project_path: project_path.clone(),
                            cause,
                        }))?
                        .starts_with(shared.project_root())
                    {
                        Ok(project_path)
                    } else {
                        Err(Error::ProjectPathInvalid(ProjectPathInvalid::OutsideOfProject {
                            ios_project_path: project_path,
                            project_path: shared.project_root().to_owned(),
                        }))
                    }
                }).unwrap_or_else(|| {
                    log::info!(
                        "`ios.project-path` not set; defaulting to {}",
                        DEFAULT_PROJECT_PATH
                    );
                    Ok(DEFAULT_PROJECT_PATH.to_owned())
                })?;
            Ok(Self {
                shared,
                development_team: raw.development_team,
                project_path,
            })
        }
    }

    fn shared(&self) -> &Shared {
        &self.shared
    }
}

impl Config {
    pub fn project_path(&self) -> PathBuf {
        self.shared.prefix_path(&self.project_path)
    }

    pub fn workspace_path(&self) -> PathBuf {
        self.project_path().join(format!(
            "{}.xcodeproj/project.xcworkspace/",
            self.shared.app_name()
        ))
    }

    pub fn export_path(&self) -> PathBuf {
        self.project_path().join("build")
    }

    pub fn export_plist_path(&self) -> PathBuf {
        self.project_path().join("ExportOptions.plist")
    }

    pub fn ipa_path(&self) -> PathBuf {
        self.export_path().join(format!("{}.ipa", self.scheme()))
    }

    pub fn app_path(&self) -> PathBuf {
        self.export_path()
            .join(format!("Payload/{}.app", self.shared.app_name()))
    }

    pub fn scheme(&self) -> String {
        format!("{}_iOS", self.shared.app_name())
    }
}
