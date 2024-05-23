use super::{
    config::{Config, Metadata},
    deps, rust_version_check,
    target::Target,
};
use crate::{
    bicycle,
    target::TargetTrait as _,
    templating::{self, Pack},
    util::{
        self,
        cli::{Report, Reportable, TextWrapper},
        ln,
    },
    DuctExpressionExt,
};
use std::path::{Path, PathBuf};

pub static TEMPLATE_PACK: &str = "xcode";

#[derive(Debug)]
pub enum Error {
    RustupFailed(std::io::Error),
    RustVersionCheckFailed(util::RustVersionError),
    DepsInstallFailed(deps::Error),
    MissingPack(templating::LookupError),
    TemplateProcessingFailed(bicycle::ProcessingError),
    AssetDirSymlinkFailed(ln::Error),
    DirectoryCreationFailed {
        path: PathBuf,
        cause: std::io::Error,
    },
    XcodegenFailed(std::io::Error),
    PodInstallFailed(std::io::Error),
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
            Self::PodInstallFailed(err) => Report::error("Failed to run `pod install`", err),
        }
    }
}

// unprefixed app_root seems pretty dangerous!!
// TODO: figure out what I meant by that
#[allow(clippy::too_many_arguments)]
pub fn gen(
    config: &Config,
    metadata: &Metadata,
    submodule_path: Option<&Path>,
    bike: &bicycle::Bicycle,
    wrapper: &TextWrapper,
    non_interactive: bool,
    skip_dev_tools: bool,
    reinstall_deps: bool,
    filter: &templating::Filter,
    skip_targets_install: bool,
) -> Result<(), Error> {
    if !skip_targets_install {
        println!("Installing iOS toolchains...");
        Target::install_all().map_err(Error::RustupFailed)?;
    }
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
    let ios_pods = metadata.ios().pods().unwrap_or_default();
    let macos_pods = metadata.macos().pods().unwrap_or_default();
    let ios_pod_options = metadata.ios().pod_options().unwrap_or_default();
    let macos_pod_options = metadata.macos().pod_options().unwrap_or_default();

    #[cfg(target_arch = "x86_64")]
    let default_archs = [String::from("arm64"), String::from("x86_64")];
    #[cfg(target_arch = "aarch64")]
    let default_archs = [String::from("arm64")];
    bike.filter_and_process(
        src,
        &dest,
        |map| {
            map.insert("file-groups", &source_dirs);
            map.insert("ios-libraries", metadata.ios().libraries());
            map.insert("ios-frameworks", metadata.ios().frameworks());
            map.insert(
                "ios-valid-archs",
                metadata.ios().valid_archs().unwrap_or(&default_archs),
            );
            #[cfg(target_arch = "aarch64")]
            map.insert("ios-sim-arch", "aarch64-apple-ios-sim");
            #[cfg(target_arch = "x86_64")]
            map.insert("ios-sim-arch", "x86_64-apple-ios");
            #[cfg(target_arch = "aarch64")]
            map.insert("macos-arch", "aarch64-apple-darwin");
            #[cfg(target_arch = "x86_64")]
            map.insert("macos-arch", "x86_64-apple-darwin");
            map.insert("ios-vendor-frameworks", metadata.ios().vendor_frameworks());
            map.insert("ios-vendor-sdks", metadata.ios().vendor_sdks());
            map.insert("macos-libraries", metadata.macos().libraries());
            map.insert("macos-frameworks", metadata.macos().frameworks());
            map.insert(
                "macos-vendor-frameworks",
                metadata.macos().vendor_frameworks(),
            );
            map.insert("macos-vendor-sdks", metadata.macos().vendor_frameworks());
            map.insert("asset-catalogs", asset_catalogs);
            map.insert("ios-pods", ios_pods);
            map.insert("macos-pods", macos_pods);
            map.insert("ios-pod-options", ios_pod_options);
            map.insert("macos-pod-options", macos_pod_options);
            map.insert(
                "ios-additional-targets",
                metadata.ios().additional_targets(),
            );
            map.insert(
                "macos-additional-targets",
                metadata.macos().additional_targets(),
            );
            map.insert("ios-pre-build-scripts", metadata.ios().pre_build_scripts());
            map.insert(
                "ios-post-compile-scripts",
                metadata.ios().post_compile_scripts(),
            );
            map.insert(
                "ios-post-build-scripts",
                metadata.ios().post_build_scripts(),
            );
            map.insert(
                "macos-pre-build-scripts",
                metadata.macos().pre_build_scripts(),
            );
            map.insert(
                "macos-post-compile-scripts",
                metadata.macos().post_compile_scripts(),
            );
            map.insert(
                "macos-post-build-scripts",
                metadata.macos().post_build_scripts(),
            );
            map.insert(
                "ios-command-line-arguments",
                metadata.ios().command_line_arguments(),
            );
            map.insert(
                "macos-command-line-arguments",
                metadata.macos().command_line_arguments(),
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
    let project_yml_path = dest.join("project.yml");
    duct::cmd("xcodegen", ["generate", "--no-env", "--spec"])
        .before_spawn(move |cmd| {
            cmd.arg(&project_yml_path);
            Ok(())
        })
        .dup_stdio()
        .run()
        .map_err(Error::XcodegenFailed)?;

    if !ios_pods.is_empty() || !macos_pods.is_empty() {
        duct::cmd(
            "pod",
            [
                "install",
                &format!("--project-directory={}", dest.display()),
            ],
        )
        .dup_stdio()
        .run()
        .map_err(Error::PodInstallFailed)?;
    }
    Ok(())
}
