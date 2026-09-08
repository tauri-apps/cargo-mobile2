use super::{
    config::{Config, Metadata},
    env::Env,
    target::Target,
};
use crate::{
    bicycle,
    os::replace_path_separator,
    target::TargetTrait as _,
    templating::{self, Pack},
    util::{
        self,
        cli::{Report, Reportable, TextWrapper},
        ln,
    },
};
use path_abs::PathOps;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub static TEMPLATE_PACK: &str = "dev-eco-studio";

#[derive(Debug)]
pub enum Error {
    RustupFailed(std::io::Error),
    MissingPack(templating::LookupError),
    TemplateProcessingFailed(bicycle::ProcessingError),
    DirectoryCreationFailed {
        path: PathBuf,
        cause: std::io::Error,
    },
    DirectoryReadFailed {
        path: PathBuf,
        cause: std::io::Error,
    },
    DirectoryRemoveFailed {
        path: PathBuf,
        cause: std::io::Error,
    },
    AssetDirSymlinkFailed(ln::Error),
    FileCopyFailed {
        src: PathBuf,
        dest: PathBuf,
        cause: std::io::Error,
    },
    AssetSourceInvalid(PathBuf),
}

impl Reportable for Error {
    fn report(&self) -> Report {
        match self {
            Self::RustupFailed(err) => {
                Report::error("Failed to `rustup` OpenHarmony toolchains", err)
            }
            Self::MissingPack(err) => {
                Report::error("Failed to locate OpenHarmony template pack", err)
            }
            Self::TemplateProcessingFailed(err) => {
                Report::error("OpenHarmony template processing failed", err)
            }
            Self::DirectoryCreationFailed { path, cause } => Report::error(
                format!(
                    "Failed to create OpenHarmony assets directory at {:?}",
                    path
                ),
                cause,
            ),
            Self::DirectoryReadFailed { path, cause } => {
                Report::error(format!("Failed to read directory at {:?}", path), cause)
            }
            Self::DirectoryRemoveFailed { path, cause } => Report::error(
                format!("Failed to remove directory directory at {:?}", path),
                cause,
            ),
            Self::AssetDirSymlinkFailed(err) => Report::error(
                "Asset dir couldn't be symlinked into OpenHarmony project",
                err,
            ),
            Self::FileCopyFailed { src, dest, cause } => Report::error(
                format!("Failed to copy file at {:?} to {:?}", src, dest),
                cause,
            ),
            Self::AssetSourceInvalid(src) => Report::error(
                format!("Asset source at {:?} invalid", src),
                "Asset sources must be either a directory or a file",
            ),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn gen(
    config: &Config,
    metadata: &Metadata,
    _env: &Env,
    bike: &bicycle::Bicycle,
    _wrapper: &TextWrapper,
    filter: &templating::Filter,
    skip_targets_install: bool,
) -> Result<(), Error> {
    if !skip_targets_install {
        println!("Installing OpenHarmony toolchains...");
        Target::install_all().map_err(Error::RustupFailed)?;
    }
    println!("Generating DevEco Studio project...");
    let src = Pack::lookup_platform(TEMPLATE_PACK)
        .map_err(Error::MissingPack)?
        .expect_local();
    let dest = config.project_dir();

    bike.filter_and_process(
        src,
        &dest,
        |map| {
            map.insert(
                "root-dir-rel",
                Path::new(&replace_path_separator(
                    util::relativize_path(
                        config.app().root_dir(),
                        config.project_dir().join("entry"),
                    )
                    .into_os_string(),
                )),
            );
            map.insert("root-dir", config.app().root_dir());
            map.insert(
                "abi-list",
                Target::all()
                    .values()
                    .map(|target| target.abi)
                    .collect::<Vec<_>>(),
            );
            map.insert("target-list", Target::all().keys().collect::<Vec<_>>());
            map.insert(
                "arch-list",
                Target::all()
                    .values()
                    .map(|target| target.arch)
                    .collect::<Vec<_>>(),
            );
            map.insert(
                "has-code",
                metadata.project_dependencies().is_some()
                    || metadata.app_dependencies().is_some()
                    || metadata.app_dependencies_platform().is_some(),
            );
            map.insert("windows", cfg!(windows));
        },
        filter.fun(),
    )
    .map_err(Error::TemplateProcessingFailed)?;

    let source_dest = dest.join("app");
    for source in metadata.app_sources() {
        let source_src = config.app().root_dir().join(source);
        let source_file = source_src
            .file_name()
            .ok_or_else(|| Error::AssetSourceInvalid(source_src.clone()))?;
        fs::copy(&source_src, source_dest.join(source_file)).map_err(|cause| {
            Error::FileCopyFailed {
                src: source_src,
                dest: source_dest.clone(),
                cause,
            }
        })?;
    }

    Ok(())
}
