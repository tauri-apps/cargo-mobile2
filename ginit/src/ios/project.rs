use crate::{config::Config, templating::template_pack, util::ln};
use into_result::{command::CommandError, IntoResult as _};
use std::{fmt, process::Command};

#[derive(Debug)]
pub enum Error {
    MissingTemplatePack { name: &'static str },
    TemplateProcessingFailed(bicycle::ProcessingError),
    AppSymlinkFailed(ln::Error),
    LibSymlinkFailed(ln::Error),
    ResourcesSymlinkFailed(ln::Error),
    ScriptChmodFailed(CommandError),
    XcodegenFailed(CommandError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::MissingTemplatePack { name } => {
                write!(f, "The {:?} template pack is missing.", name)
            }
            Error::TemplateProcessingFailed(err) => {
                write!(f, "Template processing failed: {}", err)
            }
            Error::AppSymlinkFailed(err) => write!(f, "App couldn't be symlinked: {}", err),
            Error::LibSymlinkFailed(err) => write!(f, "rust-lib couldn't be symlinked: {}", err),
            Error::ResourcesSymlinkFailed(err) => {
                write!(f, "Resources couldn't be symlinked: {}", err)
            }
            Error::ScriptChmodFailed(err) => {
                write!(f, "Failed to `chmod` \"cargo_xcode.sh\": {}", err)
            }
            Error::XcodegenFailed(err) => write!(f, "Failed to run `xcodegen`: {}", err),
        }
    }
}

// unprefixed app_root seems pretty dangerous!!
pub fn create(config: &Config, bike: &bicycle::Bicycle) -> Result<(), Error> {
    let src =
        template_pack(Some(config), "xcode_project").ok_or_else(|| Error::MissingTemplatePack {
            name: "xcode_project",
        })?;
    let dest = config.ios().project_root();
    bike.process(src, &dest, |map| config.insert_template_data(map))
        .map_err(Error::TemplateProcessingFailed)?;

    ln::force_symlink_relative(
        config.app_root().join("src"),
        &dest,
        ln::TargetStyle::Directory,
    )
    .map_err(Error::AppSymlinkFailed)?;
    ln::force_symlink_relative(
        config.app_root().join("rust-lib"),
        &dest,
        ln::TargetStyle::Directory,
    )
    .map_err(Error::LibSymlinkFailed)?;
    ln::force_symlink_relative(config.asset_path(), &dest, ln::TargetStyle::Directory)
        .map_err(Error::ResourcesSymlinkFailed)?;

    Command::new("chmod")
        .arg("+x")
        .arg(dest.join("cargo_xcode.sh"))
        .status()
        .into_result()
        .map_err(Error::ScriptChmodFailed)?;
    // Note that Xcode doesn't always reload the project nicely; reopening is
    // often necessary.
    Command::new("xcodegen")
        .args(&["generate", "--spec"])
        .arg(dest.join("project.yml"))
        .status()
        .into_result()
        .map_err(Error::XcodegenFailed)?;
    Ok(())
}
