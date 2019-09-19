use crate::config::Config;
use std::path::{Path, PathBuf};

pub fn template_pack(config: Option<&Config>, name: &str) -> Option<PathBuf> {
    fn try_path(root: impl AsRef<Path>, name: &str) -> Option<PathBuf> {
        let path = root.as_ref().join("templates").join(name);
        log::info!("checking for template pack \"{}\" at {:?}", name, path);
        Some(path).filter(|path| {
            if path.exists() {
                log::info!("found template pack \"{}\" at {:?}", name, path);
                true
            } else {
                false
            }
        })
    }

    let mut path = None;
    if let Some(config) = config {
        // first we check the user's project
        path = try_path(config.project_root(), name);
        // then we check rust-lib
        if path.is_none() {
            path = try_path(config.app_root().join("rust-lib"), name);
        }
    }
    // and then we check our internal/bundled templates
    if path.is_none() {
        path = try_path(env!("CARGO_MANIFEST_DIR"), name);
    }
    if path.is_none() {
        log::info!("template pack \"{}\" was never found", name);
    }
    path
}
