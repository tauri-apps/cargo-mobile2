use crate::config::Config as RootConfig;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RawConfig {
    project_root: String,
    development_team: String,
    targets: Option<HashMap<String, HashMap<String, String>>>,
}

#[derive(Clone, Debug)]
pub struct Config<'a> {
    root_config: &'a RootConfig,
    project_root: &'a str,
    development_team: &'a str,
}

impl<'a> Config<'a> {
    pub(crate) fn from_raw(root_config: &'a RootConfig, raw_config: &'a RawConfig) -> Self {
        if raw_config.targets.is_some() {
            log::warn!("`ios.targets` specified in {}.toml - this config key is no longer necessary, and is ignored", crate::NAME);
        }
        Self {
            root_config,
            project_root: &raw_config.project_root,
            development_team: &raw_config.development_team,
        }
    }

    pub fn project_root(&self) -> PathBuf {
        self.root_config.prefix_path(&self.project_root)
    }

    pub fn workspace_path(&self) -> PathBuf {
        self.project_root().join(format!(
            "{}.xcodeproj/project.xcworkspace/",
            self.root_config.app_name()
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
            .join(format!("Payload/{}.app", self.root_config.app_name()))
    }

    pub fn scheme(&self) -> String {
        format!("{}_iOS", self.root_config.app_name())
    }
}
