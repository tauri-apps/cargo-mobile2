use crate::{
    opts,
    util::{
        self,
        cli::{Report, TextWrapper},
        repo::{self, Repo, Status},
    },
};
use std::{
    fmt::{self, Display},
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum Error {
    NoHomeDir(util::NoHomeDir),
    XcodeSelectFailed(bossy::Error),
    StatusFailed(repo::Error),
    UpdateFailed(repo::Error),
    UuidLookupFailed(bossy::Error),
    PlistReadFailed { path: PathBuf, cause: io::Error },
    PluginsDirCreationFailed { path: PathBuf, cause: io::Error },
    PluginCopyFailed(bossy::Error),
    SpecDirCreationFailed { path: PathBuf, cause: io::Error },
    SpecCopyFailed(bossy::Error),
    MetaCopyFailed(bossy::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoHomeDir(err) => write!(f, "{}", err),
            Self::XcodeSelectFailed(err) => write!(f, "Failed to get path to Xcode.app: {}", err),
            Self::StatusFailed(err) => write!(f, "{}", err),
            Self::UpdateFailed(err) => write!(f, "{}", err),
            Self::UuidLookupFailed(err) => write!(f, "Failed to lookup Xcode UUID: {}", err),
            Self::PlistReadFailed { path, cause } => {
                write!(f, "Failed to read plist at {:?}: {}", path, cause)
            }
            Self::PluginsDirCreationFailed { path, cause } => write!(
                f,
                "Failed to create Xcode plugins directory {:?}: {}",
                path, cause
            ),
            Self::PluginCopyFailed(err) => write!(f, "Failed to copy Xcode plugin: {}", err),
            Self::SpecDirCreationFailed { path, cause } => write!(
                f,
                "Failed to create Xcode language spec directory {:?}: {}",
                path, cause
            ),
            Self::SpecCopyFailed(err) => write!(f, "Failed to copy language spec: {}", err),
            Self::MetaCopyFailed(err) => write!(f, "Failed to copy language metadata: {}", err),
        }
    }
}

fn xcode_library_dir() -> Result<PathBuf, Error> {
    util::home_dir()
        .map(|home_dir| home_dir.join("Library/Developer/Xcode"))
        .map_err(Error::NoHomeDir)
}

fn xcode_app_dir() -> Result<PathBuf, Error> {
    use std::os::unix::ffi::OsStrExt as _;
    let output = bossy::Command::impure("xcode-select")
        .with_arg("-p")
        .run_and_wait_for_output()
        .map_err(Error::XcodeSelectFailed)?;
    let path: &Path = std::ffi::OsStr::from_bytes(output.stdout()).as_ref();
    Ok(path
        .parent()
        .expect("path given by `xcode-select -p` had no parent directory")
        .to_owned())
}

// Step 1: check if installed and up-to-date
fn check_plugin(
    reinstall_deps: opts::ReinstallDeps,
    xcode_version: (u32, u32),
    repo: &Repo,
    plugins_dir: &Path,
    spec_dst: &Path,
    meta_dst: &Path,
) -> Result<Status, Error> {
    let status = if reinstall_deps.yes() {
        Status::Stale
    } else {
        let plugin_dst = plugins_dir.join("Rust.ideplugin");
        let plugin_present = plugin_dst.is_dir();
        log::info!("plugin present at {:?}: {}", plugin_dst, plugin_present);
        let spec_present = spec_dst.is_file();
        log::info!("lang spec present at {:?}: {}", spec_dst, spec_present);
        let meta_present = if xcode_version.0 >= 11 {
            let present = meta_dst.is_file();
            log::info!("lang metadata present at {:?}: {}", meta_dst, present);
            present
        } else {
            true
        };
        let all_present = plugin_present && spec_present && meta_present;
        if all_present {
            // check if anything's changed upstream
            repo.status().map_err(Error::StatusFailed)?
        } else {
            Status::Stale
        }
    };
    Ok(status)
}

// Step 2: check if uuid is supported, and prompt user to open issue if not
fn check_uuid(
    wrapper: &TextWrapper,
    xcode_version: (u32, u32),
    checkout: &Path,
    xcode_app_dir: &Path,
) -> Result<bool, Error> {
    let info_path = xcode_app_dir.join("Info");
    let uuid = bossy::Command::impure("defaults")
        .with_arg("read")
        .with_arg(info_path)
        .with_arg("DVTPlugInCompatibilityUUID")
        .run_and_wait_for_str(|s| s.trim().to_owned())
        .map_err(Error::UuidLookupFailed)?;
    let plist_path = checkout.join("Plug-ins/Rust.ideplugin/Contents/Info.plist");
    let plist = fs::read_to_string(&plist_path).map_err(|cause| Error::PlistReadFailed {
        path: plist_path,
        cause,
    })?;
    if !plist.contains(&uuid) {
        Report::action_request(
            format!(
                "Your Xcode UUID ({}, version {}.{}) isn't supported by `rust-xcode-plugin`; skipping installation",
                uuid, xcode_version.0, xcode_version.1,
            ),
            "You won't be able to set breakpoints in Xcode until this is resolved! Please open an issue at https://github.com/BrainiumLLC/rust-xcode-plugin",
        ).print(&wrapper);
        Ok(false)
    } else {
        Ok(true)
    }
}

// Step 3: install plugin!
fn run_setup(
    wrapper: &TextWrapper,
    xcode_version: (u32, u32),
    checkout: &Path,
    xcode_plugins_dir: &Path,
    xcode_spec_dir: &Path,
    spec_dst: &Path,
    meta_dst: &Path,
) -> Result<(), Error> {
    if !xcode_plugins_dir.is_dir() {
        std::fs::create_dir_all(xcode_plugins_dir).map_err(|cause| {
            Error::PluginsDirCreationFailed {
                path: xcode_plugins_dir.to_owned(),
                cause,
            }
        })?;
    }
    bossy::Command::impure("cp")
        .with_arg("-r")
        .with_arg(checkout.join("Plug-ins/Rust.ideplugin"))
        .with_arg(xcode_plugins_dir)
        .run_and_wait()
        .map_err(Error::PluginCopyFailed)?;
    let spec_src = checkout.join("Specifications/Rust.xclangspec");
    if xcode_version.0 >= 11 {
        println!("`sudo` is required to add new languages to Xcode");
        bossy::Command::impure("sudo")
            .with_arg("cp")
            .with_args(&[&spec_src, spec_dst])
            .run_and_wait()
            .map_err(Error::SpecCopyFailed)?;
    } else {
        if !xcode_spec_dir.is_dir() {
            std::fs::create_dir_all(xcode_spec_dir).map_err(|cause| {
                Error::SpecDirCreationFailed {
                    path: xcode_spec_dir.to_owned(),
                    cause,
                }
            })?;
        }
        bossy::Command::impure("cp")
            .with_args(&[&spec_src, spec_dst])
            .run_and_wait()
            .map_err(Error::SpecCopyFailed)?;
    }
    if xcode_version.0 >= 11 {
        let meta_src = checkout.join("Xcode.SourceCodeLanguage.Rust.plist");
        bossy::Command::impure("sudo")
            .with_arg("cp")
            .with_args(&[&meta_src, meta_dst])
            .run_and_wait()
            .map_err(Error::MetaCopyFailed)?;
    }
    Report::victory(
        "`rust-xcode-plugin` installed successfully!",
        "Please restart Xcode and click \"Load Bundle\" when an alert shows about `Rust.ideplugin`",
    )
    .print(&wrapper);
    Ok(())
}

// https://github.com/BrainiumLLC/rust-xcode-plugin.git
pub fn install(
    wrapper: &TextWrapper,
    reinstall_deps: opts::ReinstallDeps,
    xcode_version: (u32, u32),
) -> Result<(), Error> {
    let repo = Repo::checkouts_dir("rust-xcode-plugin").map_err(Error::NoHomeDir)?;
    let xcode_library_dir = xcode_library_dir()?;
    let xcode_plugins_dir = xcode_library_dir.join("Plug-ins");
    let xcode_app_dir = xcode_app_dir()?;
    let xcode_lang_res_dir =
        xcode_app_dir.join("SharedFrameworks/SourceModel.framework/Versions/A/Resources");
    let xcode_spec_dir = if xcode_version.0 >= 11 {
        xcode_lang_res_dir.join("LanguageSpecifications")
    } else {
        xcode_app_dir.join("Specifications")
    };
    let spec_dst = xcode_spec_dir.join("Rust.xclangspec");
    let meta_dst = xcode_lang_res_dir.join("LanguageMetadata/Xcode.SourceCodeLanguage.Rust.plist");
    let status = check_plugin(
        reinstall_deps,
        xcode_version,
        &repo,
        &xcode_plugins_dir,
        &spec_dst,
        &meta_dst,
    )?;
    log::info!("`rust-xcode-plugin` installation status: {:?}", status);
    if status.stale() {
        println!("Installing `rust-xcode-plugin`...");
        repo.update("https://github.com/BrainiumLLC/rust-xcode-plugin.git")
            .map_err(Error::UpdateFailed)?;
        if check_uuid(wrapper, xcode_version, repo.path(), &xcode_app_dir)? {
            run_setup(
                wrapper,
                xcode_version,
                repo.path(),
                &xcode_plugins_dir,
                &xcode_spec_dir,
                &spec_dst,
                &meta_dst,
            )?;
        }
    }
    Ok(())
}
