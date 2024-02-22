use crate::{
    util::{
        self,
        cli::{Report, TextWrapper},
        repo::{self, Repo},
    },
    DuctExpressionExt,
};
use std::{
    fmt::{self, Display},
    io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum Error {
    NoHomeDir(util::NoHomeDir),
    XcodeSelectFailed(std::io::Error),
    StatusFailed(repo::Error),
    UpdateFailed(repo::Error),
    UuidLookupFailed(std::io::Error),
    PlistReadFailed { path: PathBuf, cause: io::Error },
    PluginsDirCreationFailed { path: PathBuf, cause: io::Error },
    PluginCopyFailed(std::io::Error),
    SpecDirCreationFailed { path: PathBuf, cause: io::Error },
    SpecCopyFailed(std::io::Error),
    MetaCopyFailed(std::io::Error),
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

pub fn xcode_user_dir() -> Result<PathBuf, Error> {
    util::home_dir()
        .map(|home_dir| home_dir.join("Library/Developer/Xcode"))
        .map_err(Error::NoHomeDir)
}

pub fn xcode_developer_dir() -> Result<PathBuf, Error> {
    duct::cmd("xcode-select", ["-p"])
        .stderr_capture()
        .read()
        .map(|output| {
            // This output is expected to end with a newline, but we'll err on
            // the safe side and proceed gracefully if it doesn't.
            PathBuf::from(output.strip_suffix('\n').unwrap_or(&output))
        })
        .map_err(Error::XcodeSelectFailed)
}

pub fn xcode_app_dir(xcode_developer_dir: &Path) -> &Path {
    xcode_developer_dir
        .parent()
        .expect("path given by `xcode-select -p` had no parent directory")
}

#[derive(Debug)]
pub struct InstallationStatus {
    pub plugin_present: bool,
    pub lang_spec_present: bool,
    pub lang_metadata_present: bool,
    pub repo_fresh: bool,
}

impl InstallationStatus {
    pub fn perfect(&self) -> bool {
        self.plugin_present && self.lang_metadata_present && self.repo_fresh
    }
}

#[derive(Debug)]
pub struct UuidStatus {
    pub uuid: String,
    pub supported: bool,
}

impl UuidStatus {
    pub fn print_action_request(&self, wrapper: &TextWrapper, xcode_version: (u32, u32)) {
        Report::action_request(
            format!(
                "Your Xcode UUID ({}, version {}.{}) isn't supported by `rust-xcode-plugin`; skipping installation",
                self.uuid, xcode_version.0, xcode_version.1,
            ),
            "You won't be able to set breakpoints in Xcode until this is resolved! Please open an issue at https://github.com/tauri-apps/rust-xcode-plugin",
        ).print(wrapper);
    }
}

#[derive(Debug)]
pub struct Context {
    repo: Repo,
    xcode_version: (u32, u32),
    xcode_plugins_dir: PathBuf,
    xcode_app_dir: PathBuf,
    xcode_spec_dir: PathBuf,
    spec_dst: PathBuf,
    meta_dst: PathBuf,
}

impl Context {
    pub fn new(xcode_version: (u32, u32)) -> Result<Self, Error> {
        let repo = Repo::checkouts_dir("rust-xcode-plugin").map_err(Error::NoHomeDir)?;
        let xcode_user_dir = xcode_user_dir()?;
        let xcode_plugins_dir = xcode_user_dir.join("Plug-ins");
        let xcode_app_dir = xcode_developer_dir().map(|path| xcode_app_dir(&path).to_owned())?;
        let xcode_lang_res_dir =
            xcode_app_dir.join("SharedFrameworks/SourceModel.framework/Versions/A/Resources");
        let xcode_spec_dir = if xcode_version.0 >= 11 {
            xcode_lang_res_dir.join("LanguageSpecifications")
        } else {
            xcode_app_dir.join("Specifications")
        };
        let spec_dst = xcode_spec_dir.join("Rust.xclangspec");
        let meta_dst =
            xcode_lang_res_dir.join("LanguageMetadata/Xcode.SourceCodeLanguage.Rust.plist");
        Ok(Self {
            repo,
            xcode_version,
            xcode_plugins_dir,
            xcode_app_dir,
            xcode_spec_dir,
            spec_dst,
            meta_dst,
        })
    }

    // Step 1: check if installed and up-to-date
    // if so, this is the last step, unless `reinstall_deps` is enabled
    pub fn check_installation(&self) -> Result<InstallationStatus, Error> {
        let plugin_dst = self.xcode_plugins_dir.join("Rust.ideplugin");
        let plugin_present = plugin_dst.is_dir();
        log::info!("plugin present at {:?}: {}", plugin_dst, plugin_present);
        let lang_spec_present = self.spec_dst.is_file();
        log::info!(
            "lang spec present at {:?}: {}",
            self.spec_dst,
            lang_spec_present
        );
        let lang_metadata_present = if self.xcode_version.0 >= 11 {
            let present = self.meta_dst.is_file();
            log::info!("lang metadata present at {:?}: {}", self.meta_dst, present);
            present
        } else {
            true
        };
        self.repo
            .status()
            .map(|status| {
                log::info!("`rust-xcode-plugin` repo status: {:?}", status);
                InstallationStatus {
                    plugin_present,
                    lang_spec_present,
                    lang_metadata_present,
                    repo_fresh: status.fresh(),
                }
            })
            .map_err(Error::StatusFailed)
    }

    // Step 2: update checkout
    fn update_repo(&self) -> Result<(), Error> {
        self.repo
            .update(
                "https://github.com/tauri-apps/rust-xcode-plugin.git",
                "master",
            )
            .map_err(Error::UpdateFailed)
    }

    // Step 3: check if uuid is supported, and prompt user to open issue if not
    pub fn check_uuid(&self) -> Result<UuidStatus, Error> {
        let info_path = self.xcode_app_dir.join("Info");
        let uuid = duct::cmd("defaults", ["read"])
            .before_spawn(move |cmd| {
                cmd.arg(&info_path).arg("DVTPlugInCompatibilityUUID");
                Ok(())
            })
            .stderr_capture()
            .read()
            .map(|s| s.trim().to_owned())
            .map_err(Error::UuidLookupFailed)?;
        let plist_path = self
            .repo
            .path()
            .join("Plug-ins/Rust.ideplugin/Contents/Info.plist");
        let plist =
            std::fs::read_to_string(&plist_path).map_err(|cause| Error::PlistReadFailed {
                path: plist_path,
                cause,
            })?;
        let supported = plist.contains(&uuid);
        Ok(UuidStatus { uuid, supported })
    }

    // Step 4: install plugin!
    fn install(&self, wrapper: &TextWrapper) -> Result<(), Error> {
        if !self.xcode_plugins_dir.is_dir() {
            std::fs::create_dir_all(&self.xcode_plugins_dir).map_err(|cause| {
                Error::PluginsDirCreationFailed {
                    path: self.xcode_plugins_dir.to_owned(),
                    cause,
                }
            })?;
        }
        let checkout = self.repo.path();
        let ide_plugin_path = checkout.join("Plug-ins/Rust.ideplugin");
        let xcode_plugins_dir = self.xcode_plugins_dir.clone();
        duct::cmd("cp", ["-r"])
            .before_spawn(move |cmd| {
                cmd.arg(&ide_plugin_path).arg(&xcode_plugins_dir);
                Ok(())
            })
            .dup_stdio()
            .run()
            .map_err(Error::PluginCopyFailed)?;
        let spec_src = checkout.join("Specifications/Rust.xclangspec");
        if self.xcode_version.0 >= 11 {
            let spec_dst = self.spec_dst.clone();
            println!("`sudo` is required to add new languages to Xcode");
            duct::cmd("sudo", ["cp"])
                .before_spawn(move |cmd| {
                    cmd.arg(&spec_src).arg(&spec_dst);
                    Ok(())
                })
                .dup_stdio()
                .run()
                .map_err(Error::SpecCopyFailed)?;
        } else {
            if !self.xcode_spec_dir.is_dir() {
                std::fs::create_dir_all(&self.xcode_spec_dir).map_err(|cause| {
                    Error::SpecDirCreationFailed {
                        path: self.xcode_spec_dir.to_owned(),
                        cause,
                    }
                })?;
            }
            duct::cmd("cp", [&spec_src, &self.spec_dst])
                .dup_stdio()
                .run()
                .map_err(Error::SpecCopyFailed)?;
        }
        if self.xcode_version.0 >= 11 {
            let meta_src = checkout.join("Xcode.SourceCodeLanguage.Rust.plist");
            let meta_dst = self.meta_dst.clone();
            duct::cmd("sudo", ["cp"])
                .before_spawn(move |cmd| {
                    cmd.arg(&meta_src).arg(&meta_dst);
                    Ok(())
                })
                .dup_stdio()
                .run()
                .map_err(Error::MetaCopyFailed)?;
        }
        Report::victory(
            "`rust-xcode-plugin` installed successfully!",
            "Please restart Xcode and click \"Load Bundle\" when an alert shows about `Rust.ideplugin`",
        )
        .print(wrapper);
        Ok(())
    }
}

// https://github.com/tauri-apps/rust-xcode-plugin.git
pub fn install(
    wrapper: &TextWrapper,
    reinstall_deps: bool,
    xcode_version: (u32, u32),
) -> Result<(), Error> {
    let ctx = Context::new(xcode_version)?;
    if !ctx.check_installation()?.perfect() || reinstall_deps {
        println!("Installing `rust-xcode-plugin`...");
        ctx.update_repo()?;
        let uuid_status = ctx.check_uuid()?;
        if uuid_status.supported {
            ctx.install(wrapper)?;
        } else {
            uuid_status.print_action_request(wrapper, ctx.xcode_version);
        }
    }
    Ok(())
}
