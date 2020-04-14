use super::{config::Config, deps, target::Target};
use crate::{
    opts::Clobbering,
    target::TargetTrait as _,
    templating,
    util::{ln, submodule::Submodule},
};
use std::{
    fmt::{self, Display},
    path::PathBuf,
};

#[derive(Debug)]
pub enum Error {
    RustupFailed(bossy::Error),
    DepsInstallFailed(deps::Error),
    MissingPack(templating::BundledPackError),
    TemplateProcessingFailed(bicycle::ProcessingError),
    SourceDirSymlinkFailed(ln::Error),
    AssetDirSymlinkFailed(ln::Error),
    ScriptChmodFailed(bossy::Error),
    XcodegenFailed(bossy::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RustupFailed(err) => write!(f, "Failed to `rustup` iOS toolchains: {}", err),
            Self::DepsInstallFailed(err) => write!(f, "Failed to install dependencies: {}", err),
            Self::MissingPack(err) => write!(f, "{}", err),
            Self::TemplateProcessingFailed(err) => write!(f, "Template processing failed: {}", err),
            Self::SourceDirSymlinkFailed(err) => {
                write!(f, "Source dir couldn't be symlinked: {}", err)
            }
            Self::AssetDirSymlinkFailed(err) => {
                write!(f, "Asset dir couldn't be symlinked: {}", err)
            }
            Self::ScriptChmodFailed(err) => {
                write!(f, "Failed to `chmod` \"cargo-xcode.sh\": {}", err)
            }
            Self::XcodegenFailed(err) => write!(f, "Failed to run `xcodegen`: {}", err),
        }
    }
}

// unprefixed app_root seems pretty dangerous!!
pub fn gen(
    config: &Config,
    submodules: Option<&Vec<Submodule>>,
    bike: &bicycle::Bicycle,
    clobbering: Clobbering,
) -> Result<(), Error> {
    Target::install_all().map_err(Error::RustupFailed)?;

    deps::install(clobbering).map_err(Error::DepsInstallFailed)?;

    let source_dirs = std::iter::once("src".into())
        .chain(
            submodules
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|submodule| submodule.path().to_owned()),
        )
        .collect::<Vec<PathBuf>>();

    let src = templating::bundled_pack("xcode-project").map_err(Error::MissingPack)?;
    let dest = config.project_dir();
    bike.process(src, &dest, |map| {
        map.insert("file-groups", source_dirs.clone());
    })
    .map_err(Error::TemplateProcessingFailed)?;

    for source_dir in source_dirs {
        ln::force_symlink_relative(
            config.app().root_dir().join(source_dir),
            &dest,
            ln::TargetStyle::Directory,
        )
        .map_err(Error::SourceDirSymlinkFailed)?;
    }

    ln::force_symlink_relative(config.app().asset_dir(), &dest, ln::TargetStyle::Directory)
        .map_err(Error::AssetDirSymlinkFailed)?;

    bossy::Command::impure("chmod")
        .with_arg("+x")
        .with_arg(dest.join("cargo-xcode.sh"))
        .run_and_wait()
        .map_err(Error::ScriptChmodFailed)?;
    // Note that Xcode doesn't always reload the project nicely; reopening is
    // often necessary.
    bossy::Command::impure("xcodegen")
        .with_args(&["generate", "--spec"])
        .with_arg(dest.join("project.yml"))
        .run_and_wait()
        .map_err(Error::XcodegenFailed)?;
    Ok(())
}
