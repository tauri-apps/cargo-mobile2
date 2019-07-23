use super::target::Target;
use crate::config::Config as RootConfig;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct RawConfig {
    project_root: String,
    targets: BTreeMap<String, Target>,
}

#[derive(Clone, Debug)]
pub struct Config<'a> {
    root_config: &'a RootConfig,
    project_root: &'a str,
    targets: &'a BTreeMap<String, Target>,
}

impl<'a> Config<'a> {
    pub(crate) fn from_raw(root_config: &'a RootConfig, raw_config: &'a RawConfig) -> Self {
        Self {
            root_config,
            project_root: &raw_config.project_root,
            targets: &raw_config.targets,
        }
    }

    pub fn targets(&self) -> &'a BTreeMap<String, Target> {
        self.targets
    }

    pub fn abi_list(&self) -> String {
        self.targets()
            .values()
            .map(|target| format!("\"{}\"", target.abi))
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn project_path(&self) -> PathBuf {
        self.root_config
            .project_root()
            .join(&self.project_root)
            .join(self.root_config.app_name())
    }

    pub fn ndk_path(&self) -> PathBuf {
        self.project_path()
            .parent()
            .unwrap()
            .join(".ndk-toolchains")
    }
}
