use crate::CONFIG;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

lazy_static! {
    static ref ABI_LIST: String = {
        CONFIG
            .android
            .targets
            .values()
            .map(|target| format!("\"{}\"", target.abi))
            .collect::<Vec<_>>()
            .join(", ")
    };
    static ref PROJECT_PATH: PathBuf = CONFIG
        .project_root()
        .join(&CONFIG.android.project_root)
        .join(CONFIG.app_name());
    static ref NDK_PATH: PathBuf = PROJECT_PATH.parent().unwrap().join(".ndk-toolchains");
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub project_root: String,
    pub targets:      BTreeMap<String, super::Target>,
}

impl Config {
    pub fn abi_list(&self) -> &str {
        &ABI_LIST
    }

    pub fn project_path(&self) -> &Path {
        &PROJECT_PATH
    }

    pub fn ndk_path(&self) -> &Path {
        &NDK_PATH
    }
}
