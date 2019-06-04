use crate::{template, util, CONFIG};
use derive_more::From;
use std::{fs, path::Path};

#[derive(Debug, From)]
pub enum ProjectCreationError {
    TemplateProcessingError(template::ProcessingError),
    SymlinkAssetsError(util::CommandError),
    CreateDirError(std::io::Error),
}

// TODO: We should verify Android env vars / offer defaults
pub fn create() -> Result<(), ProjectCreationError> {
    let src = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/templates/android_studio_project"
    ));
    let dest = CONFIG.android.project_path();
    template::process(src, dest, |map| {
        CONFIG.insert_data(map);
        map.insert("abi_list", CONFIG.android.abi_list());
    })?;
    let dest = dest.join("app/src/main/assets/");
    fs::create_dir_all(&dest)?;
    util::relative_symlink(CONFIG.asset_path(), dest)?;
    Ok(())
}
