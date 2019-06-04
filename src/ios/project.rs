use crate::{CONFIG, template, util::{self, IntoResult}};
use derive_more::From;
use std::{path::Path, process::Command};

#[derive(Debug, From)]
pub enum ProjectCreationError {
    TemplateProcessingError(template::ProcessingError),
    SymlinkRustError(util::CommandError),
    SymlinkResourcesError(util::CommandError),
}

pub fn create() -> Result<(), ProjectCreationError> {
    let src = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/xcode_project"));
    let dest = CONFIG.ios.project_root();
    template::process(src, dest, |map| CONFIG.insert_data(map))?;

    util::relative_symlink(CONFIG.source_root(), dest)
        .map_err(ProjectCreationError::SymlinkRustError)?;
    util::relative_symlink(CONFIG.asset_path(), dest)
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
