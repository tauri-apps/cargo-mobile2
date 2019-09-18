use super::core;
use crate::util;
use heck::SnekCase as _;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize)]
pub struct SharedConfig {
    project_root: PathBuf,
    core: core::Config,
}

impl SharedConfig {
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    pub fn core(&self) -> &core::Config {
        &self.core
    }

    pub fn prefix_path(&self, path: impl AsRef<Path>) -> PathBuf {
        util::prefix_path(self.project_root(), path)
    }

    pub fn unprefix_path(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<PathBuf, util::UnprefixPathError> {
        util::unprefix_path(self.project_root(), path)
    }

    pub fn app_name(&self) -> &str {
        &self.core().app_name
    }

    pub fn app_name_snake(&self) -> String {
        self.app_name().to_snek_case()
    }

    pub fn stylized_app_name(&self) -> &str {
        self.core()
            .stylized_app_name
            .as_ref()
            .unwrap_or_else(|| &self.core().app_name)
    }

    pub fn reverse_domain(&self) -> String {
        self.core()
            .domain
            .clone()
            .split('.')
            .rev()
            .collect::<Vec<_>>()
            .join(".")
    }

    pub fn app_root(&self) -> PathBuf {
        self.prefix_path(&self.core().app_root)
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.app_root().join("Cargo.toml")
    }

    pub fn asset_path(&self) -> PathBuf {
        self.app_root().join("res")
    }

    pub(crate) fn insert_template_data(&self, map: &mut bicycle::JsonMap) {
        map.insert("app-name", self.app_name());
        map.insert("app-name-snake", self.app_name_snake());
        map.insert("stylized-app-name", self.stylized_app_name());
        map.insert("reverse-domain", self.reverse_domain());
    }
}
