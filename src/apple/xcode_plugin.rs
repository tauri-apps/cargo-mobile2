use crate::{
    opts,
    util::{
        self,
        cli::{Report, TextWrapper},
        Git,
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
    FetchFailed(bossy::Error),
    RevParseLocalFailed(bossy::Error),
    RevParseRemoteFailed(bossy::Error),
    CheckoutsDirCreationFailed { path: PathBuf, cause: io::Error },
    CloneFailed(bossy::Error),
    PullFailed(bossy::Error),
    UuidLookupFailed(bossy::Error),
    UuidInvalidUtf8(std::str::Utf8Error),
    PlistReadFailed { path: PathBuf, cause: io::Error },
    PluginsDirCreationFailed { path: PathBuf, cause: io::Error },
    PluginCopyFailed(bossy::Error),
    SpecCopyFailed(bossy::Error),
    MetaCopyFailed(bossy::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoHomeDir(err) => write!(f, "{}", err),
            Self::XcodeSelectFailed(err) => write!(f, "Failed to get path to Xcode.app: {}", err),
            Self::FetchFailed(err) => write!(f, "Failed to fetch Xcode plugin repo: {}", err),
            Self::RevParseLocalFailed(err) => {
                write!(f, "Failed to get Xcode plugin checkout revision: {}", err)
            }
            Self::RevParseRemoteFailed(err) => {
                write!(f, "Failed to get Xcode plugin upstream revision: {}", err)
            }
            Self::CheckoutsDirCreationFailed { path, cause } => write!(
                f,
                "Failed to create checkouts directory {:?}: {}",
                path, cause
            ),
            Self::CloneFailed(err) => write!(f, "Failed to clone Xcode plugin repo: {}", err),
            Self::PullFailed(err) => write!(f, "Failed to update Xcode plugin repo: {}", err),
            Self::UuidLookupFailed(err) => write!(f, "Failed to lookup Xcode UUID: {}", err),
            Self::UuidInvalidUtf8(err) => write!(f, "Xcode UUID contained invalid UTF-8: {}", err),
            Self::PlistReadFailed { path, cause } => {
                write!(f, "Failed to read plist at {:?}: {}", path, cause)
            }
            Self::PluginsDirCreationFailed { path, cause } => write!(
                f,
                "Failed to create Xcode plugins directory {:?}: {}",
                path, cause
            ),
            Self::PluginCopyFailed(err) => write!(f, "Failed to copy Xcode plugin: {}", err),
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

#[derive(Clone, Copy, Debug)]
enum Status {
    NeedsUpdate,
    PerfectlyLovely,
}

fn check_changes(checkout: &Path) -> Result<Status, Error> {
    if !checkout.is_dir() {
        Ok(Status::NeedsUpdate)
    } else {
        let git = Git::new(&checkout);
        git.command_parse("fetch origin")
            .run_and_wait()
            .map_err(Error::FetchFailed)?;
        let local = git
            .command_parse("rev-parse HEAD")
            .run_and_wait_for_output()
            .map_err(Error::RevParseLocalFailed)?;
        let remote = git
            .command_parse("rev-parse @{u}")
            .run_and_wait_for_output()
            .map_err(Error::RevParseRemoteFailed)?;
        if local.stdout() != remote.stdout() {
            Ok(Status::NeedsUpdate)
        } else {
            Ok(Status::PerfectlyLovely)
        }
    }
}

// Step 1: check if installed and up-to-date
fn check_plugin(
    clobbering: opts::Clobbering,
    checkout: &Path,
    plugins_dir: &Path,
    spec_dst: &Path,
    meta_dst: &Path,
) -> Result<Status, Error> {
    if clobbering.allowed() {
        Ok(Status::NeedsUpdate)
    } else {
        let plugin_dst = plugins_dir.join("Rust.ideplugin");
        let plugin_present = plugin_dst.is_dir();
        log::info!("plugin present at {:?}: {}", plugin_dst, plugin_present);
        let spec_present = spec_dst.is_file();
        log::info!("lang spec present at {:?}: {}", spec_dst, spec_present);
        let meta_present = meta_dst.is_file();
        log::info!("lang metadata present at {:?}: {}", meta_dst, meta_present);
        let all_present = plugin_present && spec_present && meta_present;
        if all_present {
            // check if anything's changed upstream
            check_changes(checkout)
        } else {
            Ok(Status::NeedsUpdate)
        }
    }
}

// Step 2: clone repo into temp dir
fn clone_plugin(checkouts_dir: &Path, checkout: &Path) -> Result<(), Error> {
    if !checkout.is_dir() {
        if !checkouts_dir.is_dir() {
            std::fs::create_dir_all(&checkouts_dir).map_err(|cause| {
                Error::CheckoutsDirCreationFailed {
                    path: checkouts_dir.to_owned(),
                    cause,
                }
            })?;
        }
        let git = Git::new(&checkouts_dir);
        git.command_parse("clone --depth 1 https://github.com/wyldpixel/rust-xcode-plugin.git")
            .with_arg(&checkout)
            .run_and_wait()
            .map_err(Error::CloneFailed)?;
    } else {
        let git = Git::new(&checkout);
        git.command_parse("pull --ff-only --depth 1")
            .run_and_wait()
            .map_err(Error::PullFailed)?;
    }
    Ok(())
}

// Step 3: check if uuid is supported, and prompt user to open issue if not
fn check_uuid(
    wrapper: &TextWrapper,
    interactivity: opts::Interactivity,
    checkout: &Path,
    xcode_app_dir: &Path,
) -> Result<bool, Error> {
    let info_path = xcode_app_dir.join("Info");
    let uuid_output = bossy::Command::impure("defaults")
        .with_arg("read")
        .with_arg(info_path)
        .with_arg("DVTPlugInCompatibilityUUID")
        .run_and_wait_for_output()
        .map_err(Error::UuidLookupFailed)?;
    let uuid_output = uuid_output.stdout_str().map_err(Error::UuidInvalidUtf8)?;
    let uuid = uuid_output.trim();
    let plist_path = checkout.join("Plug-ins/Rust.ideplugin/Contents/Info.plist");
    let plist = fs::read_to_string(&plist_path).map_err(|cause| Error::PlistReadFailed {
        path: plist_path,
        cause,
    })?;
    if !plist.contains(uuid) {
        let report = Report::action_request(
            format!(
                "Your Xcode UUID ({}) isn't supported by `rust-xcode-plugin`; skipping installation",
                uuid
            ),
            "You won't be able to set breakpoints in Xcode until this is resolved! Please open an issue at https://github.com/mtak-/rust-xcode-plugin",
        );
        eprintln!("{}", report.render(&wrapper, interactivity));
        Ok(false)
    } else {
        Ok(true)
    }
}

// Step 4: install plugin!
fn run_setup(
    wrapper: &TextWrapper,
    interactivity: opts::Interactivity,
    checkout: &Path,
    plugins_dir: &Path,
    spec_dst: &Path,
    meta_dst: &Path,
) -> Result<(), Error> {
    if !plugins_dir.is_dir() {
        std::fs::create_dir_all(plugins_dir).map_err(|cause| Error::PluginsDirCreationFailed {
            path: plugins_dir.to_owned(),
            cause,
        })?;
    }
    bossy::Command::impure("cp")
        .with_arg("-r")
        .with_arg(checkout.join("Plug-ins/Rust.ideplugin"))
        .with_arg(plugins_dir)
        .run_and_wait()
        .map_err(Error::PluginCopyFailed)?;
    let spec_src = checkout.join("Specifications/Rust.xclangspec");
    println!("`sudo` is required to add new languages to Xcode");
    bossy::Command::impure("sudo")
        .with_arg("cp")
        .with_args(&[&spec_src, spec_dst])
        .run_and_wait()
        .map_err(Error::SpecCopyFailed)?;
    let meta_src = checkout.join("Xcode.SourceCodeLanguage.Rust.plist");
    bossy::Command::impure("sudo")
        .with_arg("cp")
        .with_args(&[&meta_src, meta_dst])
        .run_and_wait()
        .map_err(Error::MetaCopyFailed)?;
    let report = Report::victory(
        "`rust-xcode-plugin` installed successfully!",
        "Please restart Xcode and click \"Load Bundle\" when an alert shows about `Rust.ideplugin`",
    );
    println!("{}", report.render(&wrapper, interactivity));
    Ok(())
}

pub fn install(
    wrapper: &TextWrapper,
    interactivity: opts::Interactivity,
    clobbering: opts::Clobbering,
) -> Result<(), Error> {
    let checkouts_dir = util::install_dir()
        .map_err(Error::NoHomeDir)?
        .join("checkouts");
    let checkout = checkouts_dir.join("rust-xcode-plugin");
    let xcode_library_dir = xcode_library_dir()?;
    let plugins_dir = xcode_library_dir.join("Plug-ins");
    let xcode_app_dir = xcode_app_dir()?;
    let lang_res_dir =
        xcode_app_dir.join("SharedFrameworks/SourceModel.framework/Versions/A/Resources");
    let spec_dst = lang_res_dir.join("LanguageSpecifications/Rust.xclangspec");
    let meta_dst = lang_res_dir.join("LanguageMetadata/Xcode.SourceCodeLanguage.Rust.plist");
    let status = check_plugin(clobbering, &checkout, &plugins_dir, &spec_dst, &meta_dst)?;
    log::info!("`rust-xcode-plugin` installation status: {:?}", status);
    if matches!(status, Status::NeedsUpdate) {
        clone_plugin(&checkouts_dir, &checkout)?;
        if check_uuid(wrapper, interactivity, &checkout, &xcode_app_dir)? {
            run_setup(
                wrapper,
                interactivity,
                &checkout,
                &plugins_dir,
                &spec_dst,
                &meta_dst,
            )?;
        }
    }
    Ok(())
}
