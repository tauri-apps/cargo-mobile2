use crate::{config::Config, templating::template_pack, util::ln};
use into_result::IntoResult as _;
use std::process::Command;

#[derive(Debug)]
pub enum ProjectCreationError {
    MissingTemplatePack,
    TemplateProcessingError(bicycle::ProcessingError),
    SymlinkAppError(ln::Error),
    SymlinkLibError(ln::Error),
    SymlinkResourcesError(ln::Error),
}

// unprefixed app_root seems pretty dangerous!!
pub fn create(config: &Config, bike: &bicycle::Bicycle) -> Result<(), ProjectCreationError> {
    let src = template_pack(Some(config), "xcode_project")
        .ok_or_else(|| ProjectCreationError::MissingTemplatePack)?;
    let dest = config.ios().project_root();
    bike.process(src, &dest, |map| config.insert_template_data(map))
        .map_err(ProjectCreationError::TemplateProcessingError)?;

    ln::force_symlink_relative(
        config.app_root().join("src"),
        &dest,
        ln::TargetStyle::Directory,
    )
    .map_err(ProjectCreationError::SymlinkAppError)?;
    ln::force_symlink_relative(
        config.app_root().join("rust-lib"),
        &dest,
        ln::TargetStyle::Directory,
    )
    .map_err(ProjectCreationError::SymlinkLibError)?;
    ln::force_symlink_relative(config.asset_path(), &dest, ln::TargetStyle::Directory)
        .map_err(ProjectCreationError::SymlinkResourcesError)?;

    Command::new("chmod")
        .arg("+x")
        .arg(dest.join("cargo_xcode.sh"))
        .status()
        .into_result()
        .expect("Failed to run `chmod`");
    // Note that Xcode doesn't always reload the project nicely; reopening is
    // often necessary.
    Command::new("xcodegen")
        .args(&["generate", "--spec"])
        .arg(dest.join("project.yml"))
        .status()
        .into_result()
        .expect("Failed to run `xcodegen`");
    Ok(())
}
