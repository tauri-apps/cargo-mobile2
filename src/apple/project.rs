use super::{config::Config, deps, target::Target};
use crate::{opts::Clobbering, target::TargetTrait as _, templating, util::ln};
use std::fmt::{self, Display};

#[derive(Debug)]
pub enum Error {
    RustupFailed(bossy::Error),
    DepsInstallFailed(deps::Error),
    MissingPack(templating::BundledPackError),
    TemplateProcessingFailed(bicycle::ProcessingError),
    AppSymlinkFailed(ln::Error),
    LibSymlinkFailed(ln::Error),
    ResourcesSymlinkFailed(ln::Error),
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
            Self::AppSymlinkFailed(err) => write!(f, "App couldn't be symlinked: {}", err),
            Self::LibSymlinkFailed(err) => write!(f, "rust-lib couldn't be symlinked: {}", err),
            Self::ResourcesSymlinkFailed(err) => {
                write!(f, "Resources couldn't be symlinked: {}", err)
            }
            Self::ScriptChmodFailed(err) => {
                write!(f, "Failed to `chmod` \"cargo-xcode.sh\": {}", err)
            }
            Self::XcodegenFailed(err) => write!(f, "Failed to run `xcodegen`: {}", err),
        }
    }
}

// unprefixed app_root seems pretty dangerous!!
pub fn gen(config: &Config, bike: &bicycle::Bicycle, clobbering: Clobbering) -> Result<(), Error> {
    Target::install_all().map_err(Error::RustupFailed)?;

    deps::install(clobbering).map_err(Error::DepsInstallFailed)?;

    let src = templating::bundled_pack("xcode-project").map_err(Error::MissingPack)?;
    let dest = config.project_dir();
    bike.process(src, &dest, |_| ())
        .map_err(Error::TemplateProcessingFailed)?;

    ln::force_symlink_relative(
        config.app().root_dir().join("src"),
        &dest,
        ln::TargetStyle::Directory,
    )
    .map_err(Error::AppSymlinkFailed)?;
    ln::force_symlink_relative(
        config.app().root_dir().join("rust-lib"),
        &dest,
        ln::TargetStyle::Directory,
    )
    .map_err(Error::LibSymlinkFailed)?;
    ln::force_symlink_relative(config.app().asset_dir(), &dest, ln::TargetStyle::Directory)
        .map_err(Error::ResourcesSymlinkFailed)?;

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
