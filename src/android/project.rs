use super::{config::Config, env::Env, ndk, target::Target};
use crate::{
    dot_cargo,
    target::TargetTrait as _,
    templating::{self, Pack},
    util::{
        self,
        cli::{Report, Reportable},
        ln,
    },
};
use std::{fs, path::PathBuf};

#[derive(Debug)]
pub enum Error {
    RustupFailed(bossy::Error),
    MissingPack(templating::LookupError),
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

impl Reportable for Error {
    fn report(&self) -> Report {
        match self {
            Self::RustupFailed(err) => Report::error("Failed to `rustup` Android toolchains", err),
            Self::MissingPack(err) => Report::error("Failed to locate Android template pack", err),
            Self::TemplateProcessingFailed(err) => {
                Report::error("Android template processing failed", err)
            }
            Self::DirectoryCreationFailed { path, cause } => Report::error(
                format!("Failed to create Android assets directory at {:?}", path),
                cause,
            ),
            Self::AssetDirSymlinkFailed(err) => {
                Report::error("Asset dir couldn't be symlinked into Android project", err)
            }
            Self::DotCargoLoadFailed(err) => err.report(),
            Self::DotCargoGenFailed(err) => {
                Report::error("Failed to generate Android cargo config", err)
            }
            Self::DotCargoWriteFailed(err) => err.report(),
        }
    }
}

pub fn gen(
    config: &Config,
    env: &Env,
    bike: &bicycle::Bicycle,
    filter: &templating::Filter,
) -> Result<(), Error> {
    Target::install_all().map_err(Error::RustupFailed)?;
    let src = Pack::lookup("android-studio-project")
        .map_err(Error::MissingPack)?
        .expect_local();
    let dest = config.project_dir();
    bike.filter_and_process(
        src,
        &dest,
        |map| {
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
        },
        filter.fun(),
    )
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
