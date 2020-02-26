use ginit_core::{
    bundle,
    exports::{bicycle, toml},
    util::{self, cli::NonZeroExit},
};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct Context<'a> {
    bundle_files: &'a bundle::Files,
    name: &'a str,
    src_dir: PathBuf,
}

impl<'a> Context<'a> {
    pub fn new(bundle_files: &'a bundle::Files, name: &'a str) -> Result<Self, NonZeroExit> {
        let src_dir = util::temp_dir().join("plugin-src").join(name);
        Ok(Self {
            bundle_files,
            name,
            src_dir,
        })
    }

    fn install_bin(&self) -> Result<(), NonZeroExit> {
        let dest = self.bundle_files.plugin_bin(self.name);
        todo!()
    }

    fn write_manifest(&self) -> Result<(), NonZeroExit> {
        let dest = self.bundle_files.plugin_manifest(self.name);
        todo!()
    }

    fn copy_templates(&self) -> Result<(), NonZeroExit> {
        let src = self.src_dir.join("templates");
        let dest = self.bundle_files.plugin_templates(self.name);
        let actions = bicycle::traverse(src, dest, bicycle::no_transform, None)
            .map_err(NonZeroExit::display)?;
        bicycle::Bicycle::default()
            .process_actions(actions.iter(), |_| ())
            .map_err(NonZeroExit::display)
    }

    pub fn install(self) -> Result<(), NonZeroExit> {
        let dest_dir = self.bundle_files.plugin_dir(self.name);
        fs::create_dir_all(dest_dir).map_err(NonZeroExit::display)?;
        self.install_bin()?;
        self.write_manifest()?;
        self.copy_templates()?;
        fs::remove_dir_all(self.src_dir).map_err(NonZeroExit::display)
    }
}
