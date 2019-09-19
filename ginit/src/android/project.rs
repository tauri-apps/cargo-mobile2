use super::target::Target;
use crate::{config::Config, target::TargetTrait as _, templating::template_pack, util::ln};
use std::{fmt, fs, path::PathBuf};

#[derive(Debug)]
pub enum Error {
    MissingTemplatePack {
        name: &'static str,
    },
    TemplateProcessingFailed(bicycle::ProcessingError),
    DirectoryCreationFailed {
        path: PathBuf,
        cause: std::io::Error,
    },
    AssetSymlinkFailed(ln::Error),
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
            Error::DirectoryCreationFailed { path, cause } => {
                write!(f, "Failed to create directory at {:?}: {}", path, cause)
            }
            Error::AssetSymlinkFailed(err) => write!(f, "Assets couldn't be symlinked: {}", err),
        }
    }
}

// TODO: We should verify Android env vars / offer defaults
pub fn create(config: &Config, bike: &bicycle::Bicycle) -> Result<(), Error> {
    let src = template_pack(Some(config), "android-studio-project").ok_or_else(|| {
        Error::MissingTemplatePack {
            name: "android-studio-project",
        }
    })?;
    let dest = config.android().project_path();
    bike.process(src, &dest, |map| {
        config.insert_template_data(map);
        map.insert(
            "abi-list",
            Target::all()
                .values()
                .map(|target| target.abi)
                .collect::<Vec<_>>(),
        );
        map.insert("abi-list-joined", {
            Target::all()
                .values()
                .map(|target| format!("\"{}\"", target.abi))
                .collect::<Vec<_>>()
                .join(", ")
        });
    })
    .map_err(Error::TemplateProcessingFailed)?;
    let dest = dest.join("app/src/main/assets/");
    fs::create_dir_all(&dest).map_err(|cause| Error::DirectoryCreationFailed {
        path: dest.clone(),
        cause,
    })?;
    ln::force_symlink_relative(config.asset_path(), dest, ln::TargetStyle::Directory)
        .map_err(Error::AssetSymlinkFailed)?;
    Ok(())
}
