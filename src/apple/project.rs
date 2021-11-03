use super::{
    config::{Config, Metadata},
    deps, rust_version_check,
    target::Target,
};
use crate::{
    opts,
    target::TargetTrait as _,
    templating::{self, Pack},
    util::{
        self,
        cli::{Report, Reportable, TextWrapper},
        ln,
    },
};
use std::path::{Path, PathBuf};

pub static TEMPLATE_PACK: &str = "xcode";

#[derive(Debug)]
pub enum Error {
    RustupFailed(bossy::Error),
    RustVersionCheckFailed(util::RustVersionError),
    DepsInstallFailed(deps::Error),
    MissingPack(templating::LookupError),
    TemplateProcessingFailed(bicycle::ProcessingError),
    AssetDirSymlinkFailed(ln::Error),
    DirectoryCreationFailed {
        path: PathBuf,
        cause: std::io::Error,
    },
    XcodegenFailed(bossy::Error),
}

impl Reportable for Error {
    fn report(&self) -> Report {
        match self {
            Self::RustupFailed(err) => Report::error("Failed to `rustup` Apple toolchains", err),
            Self::RustVersionCheckFailed(err) => err.report(),
            Self::DepsInstallFailed(err) => {
                Report::error("Failed to install Apple dependencies", err)
            }
            Self::MissingPack(err) => Report::error("Failed to locate Xcode template pack", err),
            Self::TemplateProcessingFailed(err) => {
                Report::error("Xcode template processing failed", err)
            }
            Self::AssetDirSymlinkFailed(err) => {
                Report::error("Asset dir couldn't be symlinked into Xcode project", err)
            }
            Self::DirectoryCreationFailed { path, cause } => Report::error(
                format!("Failed to create iOS assets directory at {:?}", path),
                cause,
            ),
            Self::XcodegenFailed(err) => Report::error("Failed to run `xcodegen`", err),
        }
    }
}

// unprefixed app_root seems pretty dangerous!!
// TODO: figure out what I meant by that
pub fn gen(
    config: &Config,
    metadata: &Metadata,
    submodule_path: Option<&Path>,
    bike: &bicycle::Bicycle,
    wrapper: &TextWrapper,
    non_interactive: opts::NonInteractive,
    skip_dev_tools: opts::SkipDevTools,
    reinstall_deps: opts::ReinstallDeps,
    filter: &templating::Filter,
) -> Result<(), Error> {
    println!("Installing iOS toolchains...");
    Target::install_all().map_err(Error::RustupFailed)?;
    rust_version_check(wrapper).map_err(Error::RustVersionCheckFailed)?;

    deps::install_all(wrapper, non_interactive, skip_dev_tools, reinstall_deps)
        .map_err(Error::DepsInstallFailed)?;

    let dest = config.project_dir();
    let rel_prefix = util::relativize_path(config.app().root_dir(), &dest);
    let source_dirs = std::iter::once("src".as_ref())
        .chain(submodule_path)
        .map(|path| rel_prefix.join(path))
        .collect::<Vec<PathBuf>>();

    let src = Pack::lookup_platform(TEMPLATE_PACK)
        .map_err(Error::MissingPack)?
        .expect_local();

    let asset_catalogs = metadata.ios().asset_catalogs().unwrap_or_default();

    bike.filter_and_process(
        src,
        &dest,
        |map| {
            map.insert("file-groups", &source_dirs);
            map.insert("ios-frameworks", metadata.ios().frameworks());
            map.insert("ios-vendor-frameworks", metadata.ios().vendor_frameworks());
            map.insert("ios-vendor-sdks", metadata.ios().vendor_sdks());
            map.insert("macos-frameworks", metadata.macos().frameworks());
            map.insert(
                "macos-vendor-frameworks",
                metadata.macos().vendor_frameworks(),
            );
            map.insert("macos-vendor-sdks", metadata.macos().vendor_frameworks());
            map.insert("asset-catalogs", asset_catalogs);
            map.insert(
                "ios-additional-targets",
                metadata.ios().additional_targets(),
            );
            map.insert(
                "macos-additional-targets",
                metadata.macos().additional_targets(),
            );
        },
        filter.fun(),
    )
    .map_err(Error::TemplateProcessingFailed)?;

    ln::force_symlink_relative(config.app().asset_dir(), &dest, ln::TargetStyle::Directory)
        .map_err(Error::AssetDirSymlinkFailed)?;

    // Create all asset catalog directories if they don't already exist
    for dir in asset_catalogs {
        std::fs::create_dir_all(dir).map_err(|cause| Error::DirectoryCreationFailed {
            path: dest.clone(),
            cause,
        })?;
    }

    // Note that Xcode doesn't always reload the project nicely; reopening is
    // often necessary.
    println!("Generating Xcode project...");
    bossy::Command::impure("xcodegen")
        .with_args(&["generate", "--spec"])
        .with_arg(dest.join("project.yml"))
        .run_and_wait()
        .map_err(Error::XcodegenFailed)?;
    Ok(())
}
