use crate::config::SharedConfig;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt, path::PathBuf, rc::Rc};

static DEFAULT_PROJECT_ROOT: &'static str = "gen/ios";

#[derive(Debug)]
pub enum Error {
    DevelopmentTeamEmpty,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::DevelopmentTeamEmpty => write!(f, "`ios.development-team` is empty."),
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
            Ok(Self {
                shared,
                development_team: raw_config.development_team,
                project_root: raw_config
                    .project_root
                    .map(|project_root| {
                        if project_root == DEFAULT_PROJECT_ROOT {
                            log::warn!("`ios.project-root` is set to the default value; you can remove it from your config");
                        }
                        project_root
                    }).unwrap_or_else(|| {
                        log::info!(
                            "`ios.project-root` not set; defaulting to {}",
                            DEFAULT_PROJECT_ROOT
                        );
                        DEFAULT_PROJECT_ROOT.to_owned()
                    }),
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
