use super::target::Target;
use crate::{config::Config, target::TargetTrait as _, templating::template_pack, util};
use into_result::command::CommandError;
use std::fs;

#[derive(Debug, derive_more::From)]
pub enum ProjectCreationError {
    MissingTemplatePack,
    TemplateProcessingError(bicycle::ProcessingError),
    SymlinkAssetsError(CommandError),
    CreateDirError(std::io::Error),
}

// TODO: We should verify Android env vars / offer defaults
pub fn create(config: &Config, bike: &bicycle::Bicycle) -> Result<(), ProjectCreationError> {
    let src = template_pack(Some(config), "android_studio_project")
        .ok_or_else(|| ProjectCreationError::MissingTemplatePack)?;
    let dest = config.android().project_path();
    bike.process(src, &dest, |map| {
        config.insert_template_data(map);
        map.insert("abi_list", {
            Target::all()
                .values()
                .map(|target| format!("\"{}\"", target.abi))
                .collect::<Vec<_>>()
                .join(", ")
        });
    })?;
    let dest = dest.join("app/src/main/assets/");
    fs::create_dir_all(&dest)?;
    util::relative_symlink(config.asset_path(), dest)?;
    Ok(())
}
