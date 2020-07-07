use super::{config::Config, deps, target::Target};
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

pub static TEMPLATE_PACK: &'static str = "xcode-project";

#[derive(Debug)]
pub enum Error {
    RustupFailed(bossy::Error),
    DepsInstallFailed(deps::Error),
    MissingPack(templating::LookupError),
    TemplateProcessingFailed(bicycle::ProcessingError),
    AssetDirSymlinkFailed(ln::Error),
    ScriptChmodFailed(bossy::Error),
    XcodegenFailed(bossy::Error),
}

impl Reportable for Error {
    fn report(&self) -> Report {
        match self {
            Self::RustupFailed(err) => Report::error("Failed to `rustup` Apple toolchains", err),
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
            Self::ScriptChmodFailed(err) => {
                Report::error("Failed to `chmod` \"cargo-xcode.sh\"", err)
            }
            Self::XcodegenFailed(err) => Report::error("Failed to run `xcodegen`", err),
        }
    }
}

// unprefixed app_root seems pretty dangerous!!
// TODO: figure out what I meant by that
pub fn gen(
    config: &Config,
    submodule_path: Option<&Path>,
    bike: &bicycle::Bicycle,
    wrapper: &TextWrapper,
    skip_dev_tools: opts::SkipDevTools,
    reinstall_deps: opts::ReinstallDeps,
    filter: &templating::Filter,
) -> Result<(), Error> {
    Target::install_all().map_err(Error::RustupFailed)?;

    deps::install(wrapper, skip_dev_tools, reinstall_deps).map_err(Error::DepsInstallFailed)?;

    let dest = config.project_dir();
    let rel_prefix = util::relativize_path(config.app().root_dir(), &dest);
    let source_dirs = std::iter::once("src".as_ref())
        .chain(submodule_path)
        .map(|path| rel_prefix.join(path))
        .collect::<Vec<PathBuf>>();

    let src = Pack::lookup(TEMPLATE_PACK)
        .map_err(Error::MissingPack)?
        .expect_local();

    bike.filter_and_process(
        src,
        &dest,
        |map| {
            map.insert("file-groups", source_dirs.clone());
        },
        filter.fun(),
    )
    .map_err(Error::TemplateProcessingFailed)?;

    ln::force_symlink_relative(config.app().asset_dir(), &dest, ln::TargetStyle::Directory)
        .map_err(Error::AssetDirSymlinkFailed)?;

    bossy::Command::impure("chmod")
        .with_arg("+x")
        .with_arg(dest.join("cargo-xcode.sh"))
        .run_and_wait()
        .map_err(Error::ScriptChmodFailed)?;
    // Note that Xcode doesn't always reload the project nicely; reopening is
    // often necessary.
    bossy::Command::impure("xcodegen")
        .with_args(&["generate", "--spec"])
        .with_arg(dest.join("project.yml"))
        .run_and_wait()
        .map_err(Error::XcodegenFailed)?;
    Ok(())
}
