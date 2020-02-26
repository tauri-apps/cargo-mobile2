use crate::exports::toml;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::{self, Display},
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum Cause {
    Missing,
    ReadFailed(io::Error),
    ParseFailed(toml::de::Error),
    MetadataInvalid(toml::de::Error),
}

#[derive(Debug)]
pub struct Error {
    path: PathBuf,
    cause: Cause,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.cause {
            Cause::Missing => write!(f, "Manifest not found: tried {:?}", self.path),
            Cause::ReadFailed(err) => {
                write!(f, "Failed to read manifest at {:?}: {}", self.path, err)
            }
            Cause::ParseFailed(err) => {
                write!(f, "Failed to parse manifest at {:?}: {}", self.path, err)
            }
            Cause::MetadataInvalid(err) => write!(
                f,
                "Failed to parse `package.metadata.ginit` at {:?}: {}",
                self.path, err
            ),
        }
    }
}

impl Error {
    fn new(path: impl Into<PathBuf>, cause: Cause) -> Self {
        Self {
            path: path.into(),
            cause,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Manifest {
    name: String,
    version: String,
    description: String,
}

impl Manifest {
    pub fn load_from_bundle(
        bundle: &super::Bundle,
        plugin: impl AsRef<str>,
    ) -> Result<Self, Error> {
        let path = bundle.plugin_manifest_path(plugin);
        let raw =
            fs::read(&path).map_err(|cause| Error::new(path.clone(), Cause::ReadFailed(cause)))?;
        toml::from_slice::<Self>(&raw).map_err(|cause| Error::new(path, Cause::ParseFailed(cause)))
    }

    pub fn load_from_cargo_toml(path: impl AsRef<Path>) -> Result<Self, Error> {
        #[derive(Debug, Deserialize, Serialize)]
        struct CargoToml {
            package: Package,
        }

        #[derive(Debug, Deserialize, Serialize)]
        struct Package {
            name: String,
            version: String,
            description: String,
            metadata: Option<HashMap<String, toml::Value>>,
        }

        #[derive(Debug, Deserialize, Serialize)]
        struct Metadata {
            name: Option<String>,
            description: Option<String>,
        }

        let path = path.as_ref();
        if !path.is_file() {
            Err(Error::new(path, Cause::Missing))
        } else {
            let raw = fs::read(path).map_err(|err| Error::new(path, Cause::ReadFailed(err)))?;
            let (name, version, description) = {
                let cargo = toml::from_slice::<CargoToml>(&raw)
                    .map_err(|err| Error::new(path, Cause::ParseFailed(err)))?;
                let metadata = cargo
                    .package
                    .metadata
                    .and_then(|metadata| {
                        metadata.get("ginit").map(|metadata| {
                            metadata
                                .clone()
                                .try_into::<Metadata>()
                                .map_err(|err| Error::new(path, Cause::MetadataInvalid(err)))
                        })
                    })
                    .transpose()?;
                let CargoToml {
                    package:
                        Package {
                            name: pkg_name,
                            version: pkg_version,
                            description: pkg_description,
                            ..
                        },
                } = cargo;
                if let Some(Metadata {
                    name: meta_name,
                    description: meta_description,
                }) = metadata
                {
                    (
                        meta_name.unwrap_or_else(|| pkg_name),
                        pkg_version,
                        meta_description.unwrap_or_else(|| pkg_description),
                    )
                } else {
                    (pkg_name, pkg_version, pkg_description)
                }
            };
            Ok(Self {
                name,
                version,
                description,
            })
        }
    }

    pub fn full_name(&self) -> &str {
        &self.name
    }

    pub fn short_name(&self) -> String {
        if self.name.starts_with("ginit-") {
            self.name.replace("ginit-", "")
        } else {
            self.name.clone()
        }
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}
