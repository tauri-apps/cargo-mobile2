use crate::{config::SharedConfig, util};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt, path::PathBuf, rc::Rc};

static DEFAULT_PROJECT_ROOT: &'static str = "gen/ios";

#[derive(Debug)]
pub enum ProjectRootInvalid {
    NormalizationFailed {
        ios_project_root: String,
        cause: util::NormalizationError,
    },
    OutsideOfProject {
        ios_project_root: String,
        project_root: PathBuf,
    },
}

impl fmt::Display for ProjectRootInvalid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NormalizationFailed {
                ios_project_root,
                cause,
            } => write!(
                f,
                "{:?} couldn't be normalized: {}",
                ios_project_root, cause
            ),
            Self::OutsideOfProject {
                ios_project_root,
                project_root,
            } => write!(
                f,
                "{:?} is outside of the project root ({:?}).",
                ios_project_root, project_root,
            ),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    DevelopmentTeamEmpty,
    ProjectRootInvalid(ProjectRootInvalid),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DevelopmentTeamEmpty => write!(f, "`ios.development-team` is empty."),
            Self::ProjectRootInvalid(err) => write!(f, "`ios.project-root` invalid: {}", err),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RawConfig {
    #[serde(alias = "development-team")]
    development_team: String,
    #[serde(alias = "project-root")]
    project_root: Option<String>,
    targets: Option<HashMap<String, HashMap<String, String>>>,
}

#[serde(rename_all = "kebab-case")]
#[derive(Clone, Debug, Serialize)]
pub struct Config {
    #[serde(skip_serializing)]
    shared: Rc<SharedConfig>,
    development_team: String,
    project_root: String,
}

impl Config {
    pub(crate) fn from_raw(shared: Rc<SharedConfig>, raw_config: RawConfig) -> Result<Self, Error> {
        if raw_config.targets.is_some() {
            log::warn!("`ios.targets` specified in {}.toml - this config key is no longer necessary, and is ignored", crate::NAME);
        }
        if raw_config.development_team.is_empty() {
            Err(Error::DevelopmentTeamEmpty)
        } else {
            let project_root = raw_config
                .project_root
                .map(|project_root| {
                    if project_root == DEFAULT_PROJECT_ROOT {
                        log::warn!("`ios.project-root` is set to the default value; you can remove it from your config");
                    }
                    if util::normalize_path(&project_root)
                        .map_err(|cause| Error::ProjectRootInvalid(ProjectRootInvalid::NormalizationFailed {
                            ios_project_root: project_root.clone(),
                            cause,
                        }))?
                        .starts_with(shared.project_root())
                    {
                        Ok(project_root)
                    } else {
                        Err(Error::ProjectRootInvalid(ProjectRootInvalid::OutsideOfProject {
                            ios_project_root: project_root,
                            project_root: shared.project_root().to_owned(),
                        }))
                    }
                }).unwrap_or_else(|| {
                    log::info!(
                        "`ios.project-root` not set; defaulting to {}",
                        DEFAULT_PROJECT_ROOT
                    );
                    Ok(DEFAULT_PROJECT_ROOT.to_owned())
                })?;
            Ok(Self {
                shared,
                development_team: raw_config.development_team,
                project_root,
            })
        }
    }

    pub fn project_root(&self) -> PathBuf {
        self.shared.prefix_path(&self.project_root)
    }

    pub fn workspace_path(&self) -> PathBuf {
        self.project_root().join(format!(
            "{}.xcodeproj/project.xcworkspace/",
            self.shared.app_name()
        ))
    }

    pub fn export_path(&self) -> PathBuf {
        self.project_root().join("build")
    }

    pub fn export_plist_path(&self) -> PathBuf {
        self.project_root().join("ExportOptions.plist")
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
