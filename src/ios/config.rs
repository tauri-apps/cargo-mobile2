use super::Target;
use crate::CONFIG;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

pub fn scheme() -> String {
    format!("{}_iOS", CONFIG.app_name())
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub project_root: String,
    pub development_team: String,
    pub targets: BTreeMap<String, Target>,
}

impl Config {
    pub fn project_root(&self) -> PathBuf {
        CONFIG.prefix_path(&self.project_root)
    }

    pub fn workspace_path(&self) -> PathBuf {
        self.project_root().join(format!(
            "{}.xcodeproj/project.xcworkspace/",
            CONFIG.app_name()
        ))
    }

    pub fn export_path(&self) -> PathBuf {
        self.project_root().join("build")
    }

    pub fn export_plist_path(&self) -> PathBuf {
        self.project_root().join("ExportOptions.plist")
    }

    pub fn ipa_path(&self) -> PathBuf {
        self.export_path().join(format!("{}.ipa", scheme()))
    }

    pub fn app_path(&self) -> PathBuf {
        self.export_path()
            .join(format!("Payload/{}.app", CONFIG.app_name()))
    }
}
