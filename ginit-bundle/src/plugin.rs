use ginit_core::bundle::manifest::{self, Manifest};
use std::path::PathBuf;

#[derive(Debug)]
pub struct Plugin {
    pub dir: PathBuf,
    pub manifest: Manifest,
}

impl Plugin {
    pub fn load(dir: impl Into<PathBuf>) -> Result<Self, manifest::Error> {
        let dir = dir.into();
        Manifest::load_from_cargo_toml(dir.join("Cargo.toml"))
            .map(|manifest| Self { dir, manifest })
    }
}
