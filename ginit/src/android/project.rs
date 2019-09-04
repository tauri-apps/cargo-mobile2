use super::target::Target;
use crate::{config::Config, target::TargetTrait as _, templating::template_pack, util::ln};
use std::{fs, path::PathBuf};

#[derive(Debug, derive_more::From)]
pub enum ProjectCreationError {
    MissingTemplatePack {
        name: &'static str,
    },
    TemplateProcessingError(bicycle::ProcessingError),
    CreateDirError {
        tried_to_create: PathBuf,
        error: std::io::Error,
    },
    SymlinkAssetsError(ln::Error),
}

// TODO: We should verify Android env vars / offer defaults
pub fn create(config: &Config, bike: &bicycle::Bicycle) -> Result<(), ProjectCreationError> {
    let src = template_pack(Some(config), "android_studio_project").ok_or_else(|| {
        ProjectCreationError::MissingTemplatePack {
            name: "android_studio_project",
        }
    })?;
    let dest = config.android().project_path();
    bike.process(src, &dest, |map| {
        config.insert_template_data(map);
        map.insert(
            "abi_list",
            Target::all()
                .values()
                .map(|target| target.abi)
                .collect::<Vec<_>>(),
        );
        map.insert("abi_list_joined", {
            Target::all()
                .values()
                .map(|target| format!("\"{}\"", target.abi))
                .collect::<Vec<_>>()
                .join(", ")
        });
    })?;
    let dest = dest.join("app/src/main/assets/");
    fs::create_dir_all(&dest).map_err(|error| ProjectCreationError::CreateDirError {
        tried_to_create: dest.clone(),
        error,
    })?;
    ln::force_symlink_relative(config.asset_path(), dest, ln::TargetStyle::Directory)?;
    Ok(())
}
