use super::{config::Config, env::Env, ndk, target::Target};
use crate::{
    dot_cargo, opts,
    target::TargetTrait as _,
    templating,
    util::{self, ln},
};
use std::{
    fmt::{self, Display},
    fs,
    path::PathBuf,
};

#[derive(Debug)]
pub enum Error {
    RustupFailed(bossy::Error),
    MissingPack(templating::BundledPackError),
    TemplateProcessingFailed(bicycle::ProcessingError),
    DirectoryCreationFailed {
        path: PathBuf,
        cause: std::io::Error,
    },
    AssetDirSymlinkFailed(ln::Error),
    DotCargoLoadFailed(dot_cargo::LoadError),
    DotCargoGenFailed(ndk::MissingToolError),
    DotCargoWriteFailed(dot_cargo::WriteError),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RustupFailed(err) => write!(f, "Failed to `rustup` Android toolchains: {}", err),
            Self::MissingPack(err) => write!(f, "{}", err),
            Self::TemplateProcessingFailed(err) => write!(f, "Template processing failed: {}", err),
            Self::DirectoryCreationFailed { path, cause } => {
                write!(f, "Failed to create directory at {:?}: {}", path, cause)
            }
            Self::AssetDirSymlinkFailed(err) => {
                write!(f, "Asset dir couldn't be symlinked: {}", err)
            }
            Self::DotCargoLoadFailed(err) => write!(f, "Failed to load cargo config: {}", err),
            Self::DotCargoGenFailed(err) => write!(f, "Failed to generate cargo config: {}", err),
            Self::DotCargoWriteFailed(err) => write!(f, "Failed to write cargo config: {}", err),
        }
    }
}

pub fn gen(
    config: &Config,
    env: &Env,
    bike: &bicycle::Bicycle,
    _clobbering: opts::Clobbering,
) -> Result<(), Error> {
    Target::install_all().map_err(Error::RustupFailed)?;
    let src = templating::bundled_pack("android-studio-project").map_err(Error::MissingPack)?;
    let dest = config.project_dir();
    bike.process(src, &dest, |map| {
        map.insert("min-sdk-version", config.min_sdk_version());
        map.insert(
            "root-dir-rel",
            util::relativize_path(config.app().root_dir(), config.project_dir()),
        );
        map.insert("targets", Target::all().values().collect::<Vec<_>>());
        map.insert("target-names", Target::all().keys().collect::<Vec<_>>());
        map.insert(
            "arches",
            Target::all()
                .values()
                .map(|target| target.arch)
                .collect::<Vec<_>>(),
        );
    })
    .map_err(Error::TemplateProcessingFailed)?;

    let dest = dest.join("app/src/main/assets/");
    fs::create_dir_all(&dest).map_err(|cause| Error::DirectoryCreationFailed {
        path: dest.clone(),
        cause,
    })?;
    ln::force_symlink_relative(config.app().asset_dir(), dest, ln::TargetStyle::Directory)
        .map_err(Error::AssetDirSymlinkFailed)?;

    {
        let mut dot_cargo =
            dot_cargo::DotCargo::load(config.app()).map_err(Error::DotCargoLoadFailed)?;
        for target in Target::all().values() {
            dot_cargo.insert_target(
                target.triple.to_owned(),
                target
                    .generate_cargo_config(config, &env)
                    .map_err(Error::DotCargoGenFailed)?,
            );
        }
        dot_cargo
            .write(config.app())
            .map_err(Error::DotCargoWriteFailed)?;
    }

    Ok(())
}
