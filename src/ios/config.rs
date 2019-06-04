use super::Target;
use crate::CONFIG;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::{Path, PathBuf}};

lazy_static! {
    static ref PROJECT_ROOT: PathBuf = CONFIG.prefix_path(&CONFIG.ios.project_root);
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub project_root: String,
    pub development_team: String,
    pub targets: BTreeMap<String, Target>,
}

impl Config {
    pub fn project_root(&self) -> &Path {
        &PROJECT_ROOT
    }
}
