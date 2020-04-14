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
    source_dirs: Vec<String>,
    ios_no_default_features: bool,
    ios_features: Vec<String>,
    macos_no_default_features: bool,
    macos_features: Vec<String>,
}

impl Config {
    pub fn from_raw(app: App, raw: Option<Raw>) -> Result<Self, Error> {
        let raw = raw.ok_or_else(|| Error::DevelopmentTeamMissing)?;

        if raw.development_team.is_empty() {
            return Err(Error::DevelopmentTeamEmpty);
        }

        let project_dir = raw
            .project_dir
            .map(|project_dir| {
                if project_dir == DEFAULT_PROJECT_DIR {
                    log::warn!("`{}.project-dir` is set to the default value; you can remove it from your config", super::NAME);
                }
                if util::under_root(&project_dir, app.root_dir())
                    .map_err(|cause| Error::ProjectDirInvalid(ProjectDirInvalid::NormalizationFailed {
                        project_dir: project_dir.clone(),
                        cause,
                    }))?
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

        let default_source_dirs = if cfg!(feature = "brainium") {
            vec!["src".to_owned(), "rust-lib".to_owned()]
        } else {
            vec!["src".to_owned()]
        };
        if raw.source_dirs.as_ref() == Some(&default_source_dirs) {
            log::warn!(
                "`{}.source-dirs` is set to the default value; you can remove it from your config",
                super::NAME
            );
        }
        let source_dirs = raw.source_dirs.unwrap_or_else(|| default_source_dirs);

        let ios_no_default_features = raw
            .ios_no_default_features
            .unwrap_or(cfg!(feature = "brainium"));
        let ios_features = raw.ios_features.unwrap_or_else(|| {
            if cfg!(target = "brainium") {
                vec!["metal".to_owned()]
            } else {
                vec![]
            }
        });

        let macos_no_default_features = raw.macos_no_default_features.unwrap_or(false);
        let macos_features = raw.macos_features.unwrap_or_else(|| {
            if cfg!(target = "brainium") {
                vec!["metal".to_owned()]
            } else {
                vec![]
            }
        });

        Ok(Self {
            app,
            development_team: raw.development_team,
            project_dir,
            source_dirs,
            ios_no_default_features,
            ios_features,
            macos_no_default_features,
            macos_features,
        })
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

    pub fn source_dirs(&self) -> &[String] {
        &self.source_dirs
    }

    pub fn ios_no_default_features(&self) -> bool {
        self.ios_no_default_features
    }

    pub fn ios_features(&self) -> &[String] {
        &self.ios_features
    }

    pub fn macos_no_default_features(&self) -> bool {
        self.macos_no_default_features
    }

    pub fn macos_features(&self) -> &[String] {
        &self.macos_features
    }
}
