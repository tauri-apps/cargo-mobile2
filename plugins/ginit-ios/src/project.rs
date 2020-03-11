use crate::{config::Config, deps, target::Target};
use ginit_core::{
    config::ConfigTrait as _,
    exports::{bicycle, bossy},
    opts::Clobbering,
    target::TargetTrait as _,
    template_pack,
    util::ln,
};
use std::fmt::{self, Display};

#[derive(Debug)]
pub enum Error {
    RustupFailed(bossy::Error),
    DepsInstallFailed(deps::Error),
    MissingTemplatePack { name: &'static str },
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
            Self::MissingTemplatePack { name } => {
                write!(f, "The {:?} template pack is missing.", name)
            }
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
pub fn generate(
    config: &Config,
    bike: &bicycle::Bicycle,
    clobbering: Clobbering,
) -> Result<(), Error> {
    Target::install_all().map_err(Error::RustupFailed)?;

    deps::install(clobbering).map_err(Error::DepsInstallFailed)?;

    let src = template_pack!(Some(config.shared()), "xcode-project").ok_or_else(|| {
        Error::MissingTemplatePack {
            name: "xcode-project",
        }
    })?;
    let dest = config.project_path();
    bike.process(src, &dest, |map| {
        config.insert_template_data(crate::NAME, map)
    })
    .map_err(Error::TemplateProcessingFailed)?;

    ln::force_symlink_relative(
        config.shared().app_root().join("src"),
        &dest,
        ln::TargetStyle::Directory,
    )
    .map_err(Error::AppSymlinkFailed)?;
    ln::force_symlink_relative(
        config.shared().app_root().join("rust-lib"),
        &dest,
        ln::TargetStyle::Directory,
    )
    .map_err(Error::LibSymlinkFailed)?;
    ln::force_symlink_relative(
        config.shared().asset_path(),
        &dest,
        ln::TargetStyle::Directory,
    )
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
