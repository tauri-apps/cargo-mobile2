use crate::{android, config::Config, init::steps::Steps, ios, target::TargetTrait};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fmt,
    fs::{self, File},
    io::{self, Write},
};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CargoTarget {
    pub ar: Option<String>,
    pub linker: Option<String>,
    pub rustflags: Vec<String>,
}

impl CargoTarget {
    fn is_empty(&self) -> bool {
        self.ar.is_none() && self.linker.is_none() && self.rustflags.is_empty()
    }
}

#[derive(Debug)]
pub enum GenError {
    AndroidEnvInvalid(android::env::Error),
}

impl fmt::Display for GenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GenError::AndroidEnvInvalid(err) => {
                write!(f, "Failed to initialize Android environment: {}", err)
            }
        }
    }
}

#[derive(Debug)]
pub enum WriteError {
    SerializationFailed(toml::ser::Error),
    DirCreationFailed(io::Error),
    OpenFailed(io::Error),
    WriteFailed(io::Error),
}

impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WriteError::SerializationFailed(err) => {
                write!(f, "Failed to serialize cargo config: {}", err)
            }
            WriteError::DirCreationFailed(err) => {
                write!(f, "Failed to create \".cargo\" directory: {}", err)
            }
            WriteError::OpenFailed(err) => {
                write!(f, "Failed to open \".cargo/config\" for writing: {}", err)
            }
            WriteError::WriteFailed(err) => {
                write!(f, "Failed to write config to \".cargo/config\": {}", err)
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CargoConfig {
    target: BTreeMap<String, CargoTarget>,
}

impl CargoConfig {
    pub fn generate(config: &Config, steps: &Steps) -> Result<Self, GenError> {
        let mut target = BTreeMap::new();
        if steps.contains(Steps::ANDROID) {
            for android_target in android::target::Target::all().values() {
                target.insert(
                    android_target.triple.to_owned(),
                    android_target.generate_cargo_config(
                        config,
                        &android::env::Env::new().map_err(GenError::AndroidEnvInvalid)?,
                    ),
                );
            }
        }
        if steps.contains(Steps::IOS) {
            for ios_target in ios::target::Target::all().values() {
                target.insert(
                    ios_target.triple.to_owned(),
                    ios_target.generate_cargo_config(),
                );
            }
        }
        target.insert(
            "x86_64-apple-darwin".to_owned(),
            CargoTarget {
                ar: None,
                linker: None,
                rustflags: vec![
                    "-C".to_owned(),
                    "target-cpu=native".to_owned(),
                    // this makes sure we'll be able to change dylib IDs
                    // (needed for dylib hot reloading)
                    "-C".to_owned(),
                    "link-arg=-headerpad_max_install_names".to_owned(),
                ],
            },
        );
        Ok(CargoConfig {
            target: target
                .into_iter()
                .filter(|(_, target)| !target.is_empty())
                .collect(),
        })
    }

    pub fn write(&self, config: &Config) -> Result<(), WriteError> {
        let serialized = toml::to_string_pretty(self).map_err(WriteError::SerializationFailed)?;
        let dir = config.prefix_path(".cargo");
        fs::create_dir_all(&dir).map_err(WriteError::DirCreationFailed)?;
        let path = dir.join("config");
        let mut file = File::create(path).map_err(WriteError::OpenFailed)?;
        file.write_all(serialized.as_bytes())
            .map_err(WriteError::WriteFailed)
    }
}
