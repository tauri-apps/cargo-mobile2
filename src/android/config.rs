use crate::CONFIG;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub project_root: String,
    pub targets: BTreeMap<String, super::Target>,
}

impl Config {
    pub fn abi_list(&self) -> String {
        self.targets
            .values()
            .map(|target| format!("\"{}\"", target.abi))
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn project_path(&self) -> PathBuf {
        CONFIG
            .project_root()
            .join(&self.project_root)
            .join(CONFIG.app_name())
    }

    pub fn ndk_path(&self) -> PathBuf {
        self.project_path()
            .parent()
            .unwrap()
            .join(".ndk-toolchains")
    }
}
