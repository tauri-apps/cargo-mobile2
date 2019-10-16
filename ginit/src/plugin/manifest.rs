use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::{self, Display},
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum Cause {
    Missing,
    OpenFailed(io::Error),
    ReadFailed(io::Error),
    ParseFailed(toml::de::Error),
    MetadataMissing,
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
            Cause::OpenFailed(err) => {
                write!(f, "Failed to open manifest at {:?}: {}", self.path, err)
            }
            Cause::ReadFailed(err) => {
                write!(f, "Failed to read manifest at {:?}: {}", self.path, err)
            }
            Cause::ParseFailed(err) => {
                write!(f, "Failed to parse manifest at {:?}: {}", self.path, err)
            }
            Cause::MetadataMissing => write!(
                f,
                "`package.metadata.ginit` section missing from manifest at {:?}.",
                self.path,
            ),
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

#[derive(Debug)]
pub struct Manifest {
    pub name: String,
    pub description: String,
    pub supports: Vec<String>,
}

impl Manifest {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Error> {
        #[derive(Debug, Deserialize, Serialize)]
        struct CargoToml {
            package: Package,
        }

        #[derive(Debug, Deserialize, Serialize)]
        struct Package {
            name: String,
            description: String,
            metadata: HashMap<String, toml::Value>,
        }

        #[derive(Debug, Deserialize, Serialize)]
        struct Metadata {
            supports: Vec<String>,
        }

        let path = path.as_ref();
        if !path.is_file() {
            Err(Error::new(path, Cause::Missing))
        } else {
            let mut file =
                File::open(path).map_err(|err| Error::new(path, Cause::OpenFailed(err)))?;
            let mut raw = Vec::new();
            file.read_to_end(&mut raw)
                .map_err(|err| Error::new(path, Cause::ReadFailed(err)))?;
            let cargo = toml::from_slice::<CargoToml>(&raw)
                .map_err(|err| Error::new(path, Cause::ParseFailed(err)))?;
            let metadata = cargo
                .package
                .metadata
                .get("ginit")
                .ok_or_else(|| Error::new(path, Cause::MetadataMissing))
                .and_then(|metadata| {
                    metadata
                        .clone()
                        .try_into::<Metadata>()
                        .map_err(|err| Error::new(path, Cause::MetadataInvalid(err)))
                })?;
            let CargoToml {
                package: Package {
                    name, description, ..
                },
            } = cargo;
            Ok(Self {
                name: if name.starts_with("ginit-") {
                    name.replace("ginit-", "")
                } else {
                    name
                },
                description,
                supports: metadata.supports,
            })
        }
    }
}
