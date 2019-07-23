use crate::{config::Config, util};
use std::{fs, path::Path};

#[derive(Debug, derive_more::From)]
pub enum ProjectCreationError {
    TemplateProcessingError(bicycle::ProcessingError),
    SymlinkAssetsError(util::CommandError),
    CreateDirError(std::io::Error),
}

// TODO: We should verify Android env vars / offer defaults
pub fn create(config: &Config, bike: &bicycle::Bicycle) -> Result<(), ProjectCreationError> {
    let src = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/templates/android_studio_project"
    ));
    let dest = config.android().project_path();
    bike.process(src, &dest, |map| {
        config.insert_template_data(map);
        map.insert("abi_list", config.android().abi_list());
    })?;
    let dest = dest.join("app/src/main/assets/");
    fs::create_dir_all(&dest)?;
    util::relative_symlink(config.asset_path(), dest)?;
    Ok(())
}
