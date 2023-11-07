use crate::util::cli::{Report, Reportable};
use serde::Deserialize;
use std::{
    fs, io,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to read {path}: {cause}")]
    ReadFailed { path: PathBuf, cause: io::Error },
    #[error("Failed to parse {path}: {cause}")]
    ParseFailed {
        path: PathBuf,
        cause: toml::de::Error,
    },
}

impl Reportable for Error {
    fn report(&self) -> Report {
        Report::error("Failed to read metadata from Cargo.toml", self)
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct Metadata {
    #[cfg(target_os = "macos")]
    #[serde(default, rename = "cargo-apple")]
    pub apple: crate::apple::config::Metadata,
    #[serde(default, rename = "cargo-android")]
    pub android: crate::android::config::Metadata,
}

impl Metadata {
    pub fn load(project_root: &Path) -> Result<Self, Error> {
        #[derive(Debug, Deserialize)]
        struct Package {
            #[serde(default)]
            metadata: Option<Metadata>,
        }

        #[derive(Debug, Deserialize)]
        struct CargoToml {
            package: Package,
        }

        let path = project_root.join("Cargo.toml");
        let toml_str = fs::read_to_string(&path).map_err(|cause| Error::ReadFailed {
            path: path.clone(),
            cause,
        })?;
        let cargo_toml = toml::from_str::<CargoToml>(&toml_str)
            .map_err(|cause| Error::ParseFailed { path, cause })?;
        Ok(cargo_toml.package.metadata.unwrap_or_default())
    }

    #[cfg(target_os = "macos")]
    pub fn apple(&self) -> &crate::apple::config::Metadata {
        &self.apple
    }

    pub fn android(&self) -> &crate::android::config::Metadata {
        &self.android
    }
}
