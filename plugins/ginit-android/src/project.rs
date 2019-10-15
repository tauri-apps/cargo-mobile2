use crate::{config::Config, target::Target};
use ginit_core::{
    config::ConfigTrait, exports::bicycle, exports::into_result::command::CommandError, opts,
    target::TargetTrait as _, template_pack, util::ln,
};
use std::{
    fmt::{self, Display},
    fs,
    path::PathBuf,
};

#[derive(Debug)]
pub enum Error {
    RustupFailed(CommandError),
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

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RustupFailed(err) => write!(f, "Failed to `rustup` Android toolchains: {}", err),
            Self::MissingTemplatePack { name } => {
                write!(f, "The {:?} template pack is missing.", name)
            }
            Self::TemplateProcessingFailed(err) => write!(f, "Template processing failed: {}", err),
            Self::DirectoryCreationFailed { path, cause } => {
                write!(f, "Failed to create directory at {:?}: {}", path, cause)
            }
            Self::AssetSymlinkFailed(err) => write!(f, "Assets couldn't be symlinked: {}", err),
        }
    }
}

pub fn generate(
    config: &Config,
    bike: &bicycle::Bicycle,
    _clobbering: opts::Clobbering,
) -> Result<(), Error> {
    Target::install_all().map_err(Error::RustupFailed)?;
    let src = template_pack!(Some(config.shared()), "android-studio-project").ok_or_else(|| {
        Error::MissingTemplatePack {
            name: "android-studio-project",
        }
    })?;
    let dest = config.project_path();
    bike.process(src, &dest, |map| {
        config.insert_template_data(crate::NAME, map);
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
    ln::force_symlink_relative(
        config.shared().asset_path(),
        dest,
        ln::TargetStyle::Directory,
    )
    .map_err(Error::AssetSymlinkFailed)?;

    Ok(())
}
