use crate::config::SharedConfig;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, rc::Rc};

const DEFAULT_MIN_SDK_VERSION: u32 = 24;
static DEFAULT_PROJECT_ROOT: &'static str = "gen/android";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct RawConfig {
    min_sdk_version: Option<u32>,
    project_root: Option<String>,
    targets: Option<HashMap<String, HashMap<String, String>>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Config {
    #[serde(skip_serializing)]
    shared: Rc<SharedConfig>,
    min_sdk_version: u32,
    project_root: String,
}

impl Config {
    pub(crate) fn from_raw(shared: Rc<SharedConfig>, raw_config: RawConfig) -> Self {
        if raw_config.targets.is_some() {
            log::warn!("`android.targets` specified in {}.toml - this config key is no longer needed, and will be ignored", crate::NAME);
        }
        Self {
            shared,
            min_sdk_version: raw_config
                .min_sdk_version
                .map(|min_sdk_version| {
                    if min_sdk_version == DEFAULT_MIN_SDK_VERSION {
                        log::warn!("`android.min_sdk_version` is set to the default value; you can remove it from your config");
                    }
                    min_sdk_version
                })
                .unwrap_or_else(|| {
                    log::info!(
                        "`android.min_sdk_version` not set; defaulting to {}",
                        DEFAULT_MIN_SDK_VERSION
                    );
                    DEFAULT_MIN_SDK_VERSION
                }),
            project_root: raw_config
                .project_root
                .map(|project_root| {
                    if project_root == DEFAULT_PROJECT_ROOT {
                        log::warn!("`android.project_root` is set to the default value; you can remove it from your config");
                    }
                    project_root
                })
                .unwrap_or_else(|| {
                    log::info!(
                        "`android.project_root` not set; defaulting to {}",
                        DEFAULT_PROJECT_ROOT
                    );
                    DEFAULT_PROJECT_ROOT.to_owned()
                }),
        }
    }

    pub fn min_sdk_version(&self) -> u32 {
        self.min_sdk_version
    }

    pub fn project_path(&self) -> PathBuf {
        self.shared
            .project_root()
            .join(&self.project_root)
            .join(self.shared.app_name())
    }
}
