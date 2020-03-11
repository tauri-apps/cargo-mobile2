pub mod global_config;
pub mod manifest;

use crate::os;
use std::{
    fmt::{self, Display},
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct NoHomeDir;

impl Display for NoHomeDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to get user's home directory!")
    }
}

#[derive(Debug)]
pub struct Storage {
    path: PathBuf,
}

impl Storage {
    pub fn new() -> Result<Self, NoHomeDir> {
        dirs::home_dir()
            .map(|home| Self {
                path: home.join(format!(".{}", crate::NAME)),
            })
            .ok_or(NoHomeDir)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn bin_path(&self) -> PathBuf {
        self.path.join(os::add_ext_to_bin_name(crate::NAME))
    }

    pub fn global_config_path(&self) -> PathBuf {
        self.path.join("global-config.toml")
    }

    pub fn plugins_dir(&self) -> PathBuf {
        self.path.join("plugins")
    }

    pub fn plugin_dir(&self, plugin: impl AsRef<str>) -> PathBuf {
        self.plugins_dir().join(plugin.as_ref())
    }

    pub fn plugin_bin_path(&self, plugin: impl AsRef<str>) -> PathBuf {
        let plugin = plugin.as_ref();
        self.plugin_dir(plugin)
            .join(os::add_ext_to_bin_name(plugin))
    }

    pub fn plugin_manifest_path(&self, plugin: impl AsRef<str>) -> PathBuf {
        let plugin = plugin.as_ref();
        self.plugin_dir(plugin).join(format!("{}.toml", plugin))
    }

    pub fn plugin_templates_dir(&self, plugin: impl AsRef<str>) -> PathBuf {
        self.plugin_dir(plugin).join("templates")
    }
}
