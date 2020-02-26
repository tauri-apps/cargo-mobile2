use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    fs, io,
    path::PathBuf,
};

#[derive(Debug)]
pub enum Error {
    ReadFailed {
        tried: PathBuf,
        cause: io::Error,
    },
    ParseFailed {
        tried: PathBuf,
        cause: toml::de::Error,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadFailed { tried, cause } => {
                write!(f, "Failed to read global config at {:?}: {}", tried, cause)
            }
            Self::ParseFailed { tried, cause } => {
                write!(f, "Failed to parse global config at {:?}: {}", tried, cause)
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GlobalConfig {
    #[serde(alias = "default-plugins")]
    pub default_plugins: Vec<String>,
}

impl GlobalConfig {
    pub fn load(bundle: &super::Bundle) -> Result<Self, Error> {
        let path = bundle.global_config_path();
        fs::read(&path)
            .map_err(|cause| Error::ReadFailed {
                tried: path.clone(),
                cause,
            })
            .and_then(|buf| {
                toml::from_slice(&buf).map_err(|cause| Error::ParseFailed { tried: path, cause })
            })
    }
}
