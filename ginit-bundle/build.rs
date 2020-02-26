use serde::Deserialize;
use std::{env, fs, path::PathBuf};

#[derive(Debug, Deserialize)]
struct Package {
    version: String,
}

#[derive(Debug, Deserialize)]
struct CargoToml {
    package: Package,
}

fn main() {
    let CargoToml {
        package: Package { version },
    } = {
        let cargo_toml_path =
            PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../ginit/Cargo.toml");
        println!("cargo:rerun-if-changed={}", cargo_toml_path.display());
        let bytes = fs::read(cargo_toml_path).expect("failed to read ginit's Cargo.toml");
        toml::from_slice::<CargoToml>(&bytes).expect("failed to parse ginit's Cargo.toml")
    };
    let content = format!("static VERSION: &'static str = {:?};\n", version);
    let version_rs_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("version.rs");
    fs::write(version_rs_path, &content).expect("failed to write version.rs");
}
