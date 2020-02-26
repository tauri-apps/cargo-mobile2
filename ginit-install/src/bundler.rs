use ginit_core::{
    exports::into_result::{
        command::{CommandError, CommandResult},
        IntoResult as _,
    },
    opts, os,
    util::CargoCommand,
};
use std::{fs, io, path::Path, process::Command};

fn bundle_name() -> String {
    include!(concat!(env!("OUT_DIR"), "/version.rs"));
    format!("{}-{}-{}", ginit_core::NAME, VERSION, os::NAME)
}

#[derive(Debug)]
pub enum Package<'a> {
    Ginit,
    Plugin(&'a str),
}

impl<'a> Package<'a> {
    fn as_str(&self) -> &'a str {
        match self {
            Self::Ginit => "ginit",
            Self::Plugin(name) => name,
        }
    }

    fn bundle_base(&self, root: &Path) -> PathBuf {
        match self {
            Self::Ginit => root.join("bundles").join(bundle_name()),
            Self::Plugin(name) => Self::bundle_base(&Self::Ginit, root)
                .join("plugins")
                .join(name),
        }
    }

    fn build_bin(&self, root: &Path, profile: opts::Profile) -> CommandResult<()> {
        CargoCommand::new("build")
            .with_package(Some(self.as_str()))
            .with_manifest_path(root.join("Cargo.toml"))
            .with_release(profile.is_release())
            .into_command_impure()
            .status()
            .into_result()
    }

    fn copy_bin(&self, root: &Path, profile: opts::Profile) -> io::Result<()> {
        let bin = os::add_ext_to_bin_name(self.as_str());
        let src = root.join("target").join(profile.as_str()).join(&bin);
        let dest = self.bundle_base(root).join("bin").join(os::NAME).join(bin);
        fs::copy(src, dest)
    }
}

// (bin, global_config, templates, plugins)
// bin = [macos, windows, linux]
// plugins = [(bin, manifest, templates)]

pub fn bundle(root: impl AsRef<Path>, profile: opts::Profile) -> io::Result<()> {
    // build bin
    let result = build_package(root, "ginit", profile);
    copy_package(root, "ginit", profile)?;
    for entry in fs::read_dir("plugins")? {
        let entry = entry?;
        let package = entry.path().file_name().unwrap();
        let result = build_package(root, package, profile);
        copy_package(root, package, profile)?;
    }
}
