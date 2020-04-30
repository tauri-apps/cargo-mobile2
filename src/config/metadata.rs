use crate::util::cli::{Report, Reportable};
use serde::Deserialize;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum Error {
    ReadFailed {
        path: PathBuf,
        cause: io::Error,
    },
    ParseFailed {
        path: PathBuf,
        cause: toml::de::Error,
    },
}

impl Reportable for Error {
    fn report(&self) -> Report {
        let msg = "Failed to read metadata from Cargo.toml";
        match self {
            Self::ReadFailed { path, cause } => {
                Report::error(msg, format!("Failed to read {:?}: {}", path, cause))
            }
            Self::ParseFailed { path, cause } => Report::error(
                msg,
                format!("Failed to parse contents of {:?}: {}", path, cause),
            ),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct Metadata {
    #[cfg(feature = "android")]
    #[serde(default, rename = "cargo-android")]
    pub android: crate::android::config::Metadata,
    #[cfg(feature = "apple")]
    #[serde(default, rename = "cargo-apple")]
    pub apple: crate::apple::config::Metadata,
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
        let bytes = fs::read(&path).map_err(|cause| Error::ReadFailed {
            path: path.clone(),
            cause,
        })?;
        let cargo_toml = toml::from_slice::<CargoToml>(&bytes)
            .map_err(|cause| Error::ParseFailed { path, cause })?;
        Ok(cargo_toml.package.metadata.unwrap_or_default())
    }
}
