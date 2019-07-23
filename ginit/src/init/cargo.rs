use crate::{android, config::Config, ios, target::TargetTrait};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Write,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct CargoConfig {
    target: BTreeMap<String, CargoTarget>,
}

impl CargoConfig {
    pub fn generate(config: &Config) -> Self {
        let mut target = BTreeMap::new();
        for android_target in android::target::Target::all(config).values() {
            target.insert(
                android_target.triple.clone(),
                android_target.generate_cargo_config(config),
            );
        }
        for ios_target in ios::target::Target::all(config).values() {
            target.insert(
                ios_target.triple.clone(),
                ios_target.generate_cargo_config(),
            );
        }
        target.insert(
            "x86_64-apple-darwin".to_owned(),
            CargoTarget {
                ar: None,
                linker: None,
                rustflags: vec!["-C".to_owned(), "target-cpu=native".to_owned()],
            },
        );
        CargoConfig { target }
    }

    pub fn write(&self, config: &Config) {
        let serialized = toml::to_string_pretty(self).expect("Failed to serialize cargo config");
        let dir = config.prefix_path(".cargo");
        fs::create_dir_all(&dir).expect("Failed to create `.cargo` directory");
        let path = dir.join("config");
        let mut file = File::create(path).expect("Failed to create cargo config file");
        file.write_all(serialized.as_bytes())
            .expect("Failed to write to cargo config file");
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CargoTarget {
    pub ar: Option<String>,
    pub linker: Option<String>,
    pub rustflags: Vec<String>,
}
