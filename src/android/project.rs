use super::{
    config::{Config, Metadata},
    env::Env,
    ndk,
    target::Target,
};
use crate::{
    android::{config::DEFAULT_VULKAN_VALIDATION, DEFAULT_ACTIVITY, DEFAULT_THEME_PARENT},
    bicycle, dot_cargo,
    os::{self, replace_path_separator},
    target::TargetTrait as _,
    templating::{self, Pack},
    util::{
        self,
        cli::{Report, Reportable, TextWrapper},
        ln, prefix_path,
    },
};
use path_abs::PathOps;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub static TEMPLATE_PACK: &str = "android-studio";
pub static ASSET_PACK_TEMPLATE_PACK: &str = "android-studio-asset-pack";

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
    DotCargoGenFailed(ndk::MissingToolError),
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
            Self::RustupFailed(err) => Report::error("Failed to `rustup` Android toolchains", err),
            Self::MissingPack(err) => Report::error("Failed to locate Android template pack", err),
            Self::TemplateProcessingFailed(err) => {
                Report::error("Android template processing failed", err)
            }
            Self::DirectoryCreationFailed { path, cause } => Report::error(
                format!("Failed to create Android assets directory at {:?}", path),
                cause,
            ),
            Self::DirectoryReadFailed { path, cause } => {
                Report::error(format!("Failed to read directory at {:?}", path), cause)
            }
            Self::DirectoryRemoveFailed { path, cause } => Report::error(
                format!("Failed to remove directory directory at {:?}", path),
                cause,
            ),
            Self::AssetDirSymlinkFailed(err) => {
                Report::error("Asset dir couldn't be symlinked into Android project", err)
            }
            Self::DotCargoGenFailed(err) => {
                Report::error("Failed to generate Android cargo config", err)
            }
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
    env: &Env,
    bike: &bicycle::Bicycle,
    wrapper: &TextWrapper,
    filter: &templating::Filter,
    dot_cargo: &mut dot_cargo::DotCargo,
    skip_targets_install: bool,
) -> Result<(), Error> {
    if !skip_targets_install {
        println!("Installing Android toolchains...");
        Target::install_all().map_err(Error::RustupFailed)?;
    }
    println!("Generating Android Studio project...");
    let src = Pack::lookup_platform(TEMPLATE_PACK)
        .map_err(Error::MissingPack)?
        .expect_local();
    let dest = config.project_dir();

    let asset_packs = metadata.asset_packs().unwrap_or_default();
    bike.filter_and_process(
        src,
        &dest,
        |map| {
            map.insert(
                "root-dir-rel",
                Path::new(&replace_path_separator(
                    util::relativize_path(config.app().root_dir(), config.project_dir())
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
            map.insert("android-app-plugins", metadata.app_plugins());
            map.insert(
                "android-project-dependencies",
                metadata.project_dependencies().unwrap_or_default(),
            );
            map.insert(
                "android-app-dependencies",
                metadata.app_dependencies().unwrap_or_default(),
            );
            map.insert(
                "android-app-dependencies-platform",
                metadata.app_dependencies_platform().unwrap_or_default(),
            );
            map.insert(
                "has-code",
                metadata.project_dependencies().is_some()
                    || metadata.app_dependencies().is_some()
                    || metadata.app_dependencies_platform().is_some(),
            );
            map.insert(
                "android-app-activity-name",
                metadata.app_activity_name().unwrap_or(DEFAULT_ACTIVITY),
            );
            map.insert(
                "android-vulkan-validation",
                metadata
                    .vulkan_validation()
                    .unwrap_or(DEFAULT_VULKAN_VALIDATION),
            );
            map.insert("android-app-permissions", metadata.app_permissions());
            map.insert(
                "android-app-theme-parent",
                metadata.app_theme_parent().unwrap_or(DEFAULT_THEME_PARENT),
            );
            let asset_packs = asset_packs
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>();
            map.insert("has-asset-packs", !asset_packs.is_empty());
            map.insert("asset-packs", asset_packs);
            map.insert("windows", cfg!(windows));
        },
        filter.fun(),
    )
    .map_err(Error::TemplateProcessingFailed)?;
    if !asset_packs.is_empty() {
        Report::action_request(
            "When running from Android Studio, you must first set your deployment option to \"APK from app bundle\".", 
            "Android Studio will not be able to find your asset packs otherwise. The option can be found under \"Run > Edit Configurations > Deploy\"."
        ).print(wrapper);
    }

    let asset_pack_src = Pack::lookup_platform(ASSET_PACK_TEMPLATE_PACK)
        .map_err(Error::MissingPack)?
        .expect_local();
    for asset_pack in asset_packs {
        bike.filter_and_process(
            &asset_pack_src,
            dest.join(&asset_pack.name),
            |map| {
                map.insert("pack-name", &asset_pack.name);
                map.insert("delivery-type", &asset_pack.delivery_type);
            },
            filter.fun(),
        )
        .map_err(Error::TemplateProcessingFailed)?;
    }

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

    let dest = prefix_path(dest, "app/src/main/");
    fs::create_dir_all(&dest).map_err(|cause| Error::DirectoryCreationFailed {
        path: dest.clone(),
        cause,
    })?;
    os::ln::force_symlink_relative(config.app().asset_dir(), dest, ln::TargetStyle::Directory)
        .map_err(Error::AssetDirSymlinkFailed)?;

    {
        for target in Target::all().values() {
            dot_cargo.insert_target(
                target.triple.to_owned(),
                target
                    .generate_cargo_config(config, env)
                    .map_err(Error::DotCargoGenFailed)?,
            );
        }
    }

    Ok(())
}
