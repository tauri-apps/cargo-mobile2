use super::global;
use heck::SnekCase as _;
use serde::Serialize;
use std::{
    fmt,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum UnprefixPathError {
    PathNotPrefixed,
}

impl fmt::Display for UnprefixPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnprefixPathError::PathNotPrefixed => write!(
                f,
                "Attempted to remove the project path prefix from a path that wasn't in the project."
            ),
        }
    }
}

pub fn prefix_path(project_root: impl AsRef<Path>, path: impl AsRef<Path>) -> PathBuf {
    project_root.as_ref().join(path)
}

pub fn unprefix_path(
    project_root: impl AsRef<Path>,
    path: impl AsRef<Path>,
) -> Result<PathBuf, UnprefixPathError> {
    path.as_ref()
        .strip_prefix(project_root)
        .map(|path| path.to_owned())
        .map_err(|_| UnprefixPathError::PathNotPrefixed)
}

#[derive(Clone, Debug, Serialize)]
pub struct SharedConfig {
    pub(super) project_root: PathBuf,
    pub(super) global: global::Config,
}

impl SharedConfig {
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    pub fn global(&self) -> &global::Config {
        &self.global
    }

    pub fn prefix_path(&self, path: impl AsRef<Path>) -> PathBuf {
        prefix_path(self.project_root(), path)
    }

    pub fn unprefix_path(&self, path: impl AsRef<Path>) -> Result<PathBuf, UnprefixPathError> {
        unprefix_path(self.project_root(), path)
    }

    pub fn app_name(&self) -> &str {
        &self.global().app_name
    }

    pub fn app_name_snake(&self) -> String {
        self.app_name().to_snek_case()
    }

    pub fn stylized_app_name(&self) -> &str {
        self.global()
            .stylized_app_name
            .as_ref()
            .unwrap_or_else(|| &self.global().app_name)
    }

    pub fn reverse_domain(&self) -> String {
        self.global()
            .domain
            .clone()
            .split('.')
            .rev()
            .collect::<Vec<_>>()
            .join(".")
    }

    pub fn app_root(&self) -> PathBuf {
        self.prefix_path(&self.global().app_root)
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.app_root().join("Cargo.toml")
    }

    pub fn asset_path(&self) -> PathBuf {
        self.app_root().join("res")
    }
}
