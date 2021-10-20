use super::{
    config::{Config, Metadata},
    env::Env,
    ndk,
    target::Target,
};
use crate::{
    dot_cargo,
    target::TargetTrait as _,
    templating::{self, Pack},
    util::{
        self,
        cli::{Report, Reportable, TextWrapper},
        ln,
    },
};
use std::{fs, path::PathBuf};

pub static TEMPLATE_PACK: &str = "android-studio";
pub static ASSET_PACK_TEMPLATE_PACK: &str = "android-studio-asset-pack";

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
    DotCargoGenFailed(ndk::MissingToolError),
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
            Self::DotCargoGenFailed(err) => {
                Report::error("Failed to generate Android cargo config", err)
            }
        }
    }
}

pub fn gen(
    config: &Config,
    metadata: &Metadata,
    env: &Env,
    bike: &bicycle::Bicycle,
    wrapper: &TextWrapper,
    filter: &templating::Filter,
    dot_cargo: &mut dot_cargo::DotCargo,
) -> Result<(), Error> {
    println!("Installing Android toolchains...");
    Target::install_all().map_err(Error::RustupFailed)?;
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
            map.insert(
                "asset-packs",
                asset_packs
                    .iter()
                    .map(|p| p.name.as_str())
                    .collect::<Vec<_>>(),
            );
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

    let dest = dest.join("app/src/main/assets/");
    fs::create_dir_all(&dest).map_err(|cause| Error::DirectoryCreationFailed {
        path: dest.clone(),
        cause,
    })?;
    ln::force_symlink_relative(config.app().asset_dir(), dest, ln::TargetStyle::Directory)
        .map_err(Error::AssetDirSymlinkFailed)?;

    {
        for target in Target::all().values() {
            dot_cargo.insert_target(
                target.triple.to_owned(),
                target
                    .generate_cargo_config(config, &env)
                    .map_err(Error::DotCargoGenFailed)?,
            );
        }
    }

    Ok(())
}
