use crate::config::Config as RootConfig;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

const DEFAULT_MIN_SDK_VERSION: u32 = 24;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct RawConfig {
    min_sdk_version: Option<u32>,
    project_root: String,
    targets: Option<HashMap<String, HashMap<String, String>>>,
}

#[derive(Clone, Debug)]
pub struct Config<'a> {
    root_config: &'a RootConfig,
    min_sdk_version: Option<u32>,
    project_root: &'a str,
}

impl<'a> Config<'a> {
    pub(crate) fn from_raw(root_config: &'a RootConfig, raw_config: &'a RawConfig) -> Self {
        if raw_config.targets.is_some() {
            log::warn!("`android.targets` specified in {}.toml - this config key is no longer necessary, and is ignored", crate::NAME);
        }
        Self {
            root_config,
            min_sdk_version: raw_config.min_sdk_version,
            project_root: &raw_config.project_root,
        }
    }

    pub fn min_sdk_version(&self) -> u32 {
        self.min_sdk_version.unwrap_or_else(|| {
            log::info!(
                "`android.min_sdk_version` not set; defaulting to {}",
                DEFAULT_MIN_SDK_VERSION
            );
            DEFAULT_MIN_SDK_VERSION
        })
    }

    pub fn project_path(&self) -> PathBuf {
        self.root_config
            .project_root()
            .join(&self.project_root)
            .join(self.root_config.app_name())
    }
}
