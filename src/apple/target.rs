use super::{
    config::{Config, Metadata},
    deps::xcode_plugin::xcode_developer_dir,
    device::Device,
    system_profile::{self, DeveloperTools},
    AuthCredentials,
};
use crate::{
    apple::project::list_destinations,
    env::{Env, ExplicitEnv as _},
    opts::{self, NoiseLevel, Profile},
    target::TargetTrait,
    util::{
        self,
        cli::{Report, Reportable},
        CargoCommand, WithWorkingDirError,
    },
    DuctExpressionExt,
};
use once_cell_regex::exports::once_cell::sync::OnceCell;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashMap},
    ffi::{OsStr, OsString},
    io::Cursor,
    process::Command,
};
use thiserror::Error;

fn verbosity(noise_level: opts::NoiseLevel) -> Option<&'static str> {
    if noise_level.pedantic() {
        None
    } else {
        Some("-quiet")
    }
}

#[derive(Debug, Error)]
pub enum VersionCheckError {
    #[error("Failed to lookup Xcode version: {0}")]
    LookupFailed(system_profile::Error),
    #[error(
        "Installed Xcode version too low ({msg} Xcode {}.{}; you have Xcode {}.{}.); please upgrade and try again",
        .you_need.0, .you_need.1,
        .you_have.0, .you_have.1
    )]
    TooLow {
        msg: &'static str,
        you_have: (u32, u32),
        you_need: (u32, u32),
    },
}

impl Reportable for VersionCheckError {
    fn report(&self) -> Report {
        match self {
            Self::LookupFailed(err) => Report::error("Failed to lookup Xcode version", err),
            Self::TooLow {
                msg,
                you_have,
                you_need,
            } => Report::action_request(
                "Installed Xcode version too low; please upgrade and try again",
                format!(
                    "{} Xcode {}.{}; you have Xcode {}.{}.",
                    msg, you_need.0, you_need.1, you_have.0, you_have.1
                ),
            ),
        }
    }
}

#[derive(Debug)]
pub enum CheckError {
    VersionCheckFailed(VersionCheckError),
    CargoCheckFailed(std::io::Error),
}

impl Reportable for CheckError {
    fn report(&self) -> Report {
        match self {
            Self::VersionCheckFailed(err) => err.report(),
            Self::CargoCheckFailed(err) => Report::error("Failed to run `cargo check`", err),
        }
    }
}

#[derive(Debug, Error)]
pub enum CompileLibError {
    #[error(transparent)]
    VersionCheckFailed(VersionCheckError),
    #[error("Failed to run `cargo build`: {0}")]
    CargoBuildFailed(std::io::Error),
}

impl Reportable for CompileLibError {
    fn report(&self) -> Report {
        match self {
            Self::VersionCheckFailed(err) => err.report(),
            Self::CargoBuildFailed(err) => Report::error("Failed to run `cargo build`", err),
        }
    }
}

#[derive(Debug, Error)]
pub enum SdkError {
    #[error("failed to find xcode path: {0}")]
    XcodePath(String),
    #[error("failed to parse Xcode SDKSettings.plist: {0}")]
    ParseSdkSettings(plist::Error),
    #[error("SDKSettings.plist missing Version")]
    MissingSdkVersion,
    #[error("{context}: {error}")]
    Io {
        context: &'static str,
        error: std::io::Error,
    },
    #[error("failed to parse installed runtimes: {0}")]
    ParseRuntimes(serde_json::Error),
    #[error("Xcode Simulator SDK {version} is not installed, please open Xcode")]
    SdkNotInstalled { version: String },
}

impl Reportable for SdkError {
    fn report(&self) -> Report {
        Report::error("Failed to validate SDK installation", self.to_string())
    }
}

#[derive(Debug, Error)]
pub enum BuildError {
    #[error("{context}: {error}")]
    Io {
        context: &'static str,
        error: std::io::Error,
    },
    #[error(transparent)]
    Sdk(#[from] SdkError),
}

impl Reportable for BuildError {
    fn report(&self) -> Report {
        Report::error("Failed to build via `xcodebuild`", self.to_string())
    }
}

#[derive(Debug, Error)]
pub enum ArchiveError {
    #[error("Failed to set app version number: {0}")]
    SetVersionFailed(WithWorkingDirError<std::io::Error>),
    #[error("Failed to archive via `xcodebuild`: {error}")]
    ArchiveFailed { error: std::io::Error },
    #[error(transparent)]
    Sdk(#[from] SdkError),
}

impl Reportable for ArchiveError {
    fn report(&self) -> Report {
        match self {
            Self::SetVersionFailed(err) => Report::error("Failed to set app version number", err),
            Self::ArchiveFailed { error } => {
                Report::error("Failed to archive via `xcodebuild`", error)
            }
            Self::Sdk(err) => Report::error("SDK validation failed", err.to_string()),
        }
    }
}

#[derive(Debug, Error)]
#[error(transparent)]
pub struct ExportError(#[from] std::io::Error);

impl Reportable for ExportError {
    fn report(&self) -> Report {
        Report::error("Failed to export archive via `xcodebuild`", &self.0)
    }
}

#[derive(Default)]
pub struct XcodebuildOptions {
    allow_provisioning_updates: bool,
    skip_codesign: bool,
    authentication_credentials: Option<AuthCredentials>,
}

impl XcodebuildOptions {
    fn args_for(&self, cmd: &mut Command) {
        if self.skip_codesign {
            cmd.args([
                "CODE_SIGNING_REQUIRED=NO",
                "CODE_SIGNING_ALLOWED=NO",
                "CODE_SIGN_IDENTITY=\"\"",
                "CODE_SIGN_ENTITLEMENTS=\"\"",
            ]);
        }

        if self.allow_provisioning_updates {
            cmd.arg("-allowProvisioningUpdates");
        }

        if let Some(credentials) = &self.authentication_credentials {
            cmd.args(["-authenticationKeyID", &credentials.key_id])
                .arg("-authenticationKeyPath")
                .arg(&credentials.key_path)
                .args(["-authenticationKeyIssuerID", &credentials.key_issuer_id]);
        }
    }
}

#[derive(Default)]
pub struct ExportConfig {
    xcodebuild_options: XcodebuildOptions,
}

impl ExportConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn allow_provisioning_updates(mut self) -> Self {
        self.xcodebuild_options.allow_provisioning_updates = true;
        self
    }

    pub fn authentication_credentials(mut self, credentials: AuthCredentials) -> Self {
        self.xcodebuild_options
            .authentication_credentials
            .replace(credentials);
        self
    }
}

#[derive(Default)]
pub struct BuildConfig {
    xcodebuild_options: XcodebuildOptions,
}

impl BuildConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn allow_provisioning_updates(mut self) -> Self {
        self.xcodebuild_options.allow_provisioning_updates = true;
        self
    }

    pub fn skip_codesign(mut self) -> Self {
        self.xcodebuild_options.skip_codesign = true;
        self
    }

    pub fn authentication_credentials(mut self, credentials: AuthCredentials) -> Self {
        self.xcodebuild_options
            .authentication_credentials
            .replace(credentials);
        self
    }
}

#[derive(Default)]
pub struct ArchiveConfig {
    xcodebuild_options: XcodebuildOptions,
}

impl ArchiveConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn allow_provisioning_updates(mut self) -> Self {
        self.xcodebuild_options.allow_provisioning_updates = true;
        self
    }

    pub fn skip_codesign(mut self) -> Self {
        self.xcodebuild_options.skip_codesign = true;
        self
    }

    pub fn authentication_credentials(mut self, credentials: AuthCredentials) -> Self {
        self.xcodebuild_options
            .authentication_credentials
            .replace(credentials);
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Target<'a> {
    pub triple: &'a str,
    pub arch: &'a str,
    pub sdk: &'a str,
    alias: Option<&'a str>,
    min_xcode_version: Option<((u32, u32), &'static str)>,
}

impl<'a> TargetTrait<'a> for Target<'a> {
    const DEFAULT_KEY: &'static str = "aarch64";

    fn all() -> &'a BTreeMap<&'a str, Self> {
        static TARGETS: OnceCell<BTreeMap<&'static str, Target<'static>>> = OnceCell::new();
        TARGETS.get_or_init(|| {
            let mut targets = BTreeMap::new();
            targets.insert(
                "aarch64",
                Target {
                    triple: "aarch64-apple-ios",
                    arch: "arm64",
                    sdk: "iphoneos",
                    alias: Some("arm64e"),
                    min_xcode_version: None,
                },
            );
            targets.insert(
                "x86_64",
                Target {
                    triple: "x86_64-apple-ios",
                    arch: "x86_64",
                    sdk: "iphonesimulator",
                    alias: None,
                    // Simulator only supports Metal as of Xcode 11.0:
                    // https://developer.apple.com/documentation/metal/developing_metal_apps_that_run_in_simulator?language=objc
                    // While this doesn't matter if you aren't using Metal,
                    // it should be fine to be opinionated about this given
                    // OpenGL's deprecation.
                    min_xcode_version: Some(((11, 0), "iOS Simulator doesn't support Metal until")),
                },
            );
            targets.insert(
                "aarch64-sim",
                Target {
                    triple: "aarch64-apple-ios-sim",
                    arch: "arm64-sim",
                    sdk: "iphonesimulator",
                    alias: Some("arm64e-sim"),
                    min_xcode_version: None,
                },
            );
            targets
        })
    }

    fn name_list() -> Vec<&'a str> {
        Self::all().keys().copied().collect::<Vec<_>>()
    }

    fn triple(&'a self) -> &'a str {
        self.triple
    }

    fn arch(&'a self) -> &'a str {
        self.arch
    }
}

impl<'a> Target<'a> {
    // TODO: Make this cleaner
    pub fn macos() -> Self {
        Self {
            triple: "x86_64-apple-darwin",
            arch: "x86_64",
            sdk: "macos",
            alias: None,
            min_xcode_version: None,
        }
    }

    pub fn is_macos(&self) -> bool {
        *self == Self::macos()
    }

    pub fn for_arch(arch: &str) -> Option<&'a Self> {
        Self::all()
            .values()
            .find(|target| target.arch == arch || target.alias == Some(arch))
    }

    fn min_xcode_version_satisfied(&self) -> Result<(), VersionCheckError> {
        self.min_xcode_version
            .map(|(min_version, msg)| {
                let tool_info = DeveloperTools::new().map_err(VersionCheckError::LookupFailed)?;
                let installed_version = tool_info.version;
                if installed_version >= min_version {
                    Ok(())
                } else {
                    Err(VersionCheckError::TooLow {
                        msg,
                        you_have: installed_version,
                        you_need: min_version,
                    })
                }
            })
            .unwrap_or_else(|| Ok(()))
    }

    fn cargo(
        &'a self,
        config: &'a Config,
        metadata: &'a Metadata,
        subcommand: &'a str,
    ) -> Result<CargoCommand<'a>, VersionCheckError> {
        let metadata = if self.is_macos() {
            metadata.macos()
        } else {
            metadata.ios()
        };
        self.min_xcode_version_satisfied().map(|()| {
            CargoCommand::new(subcommand)
                .with_package(Some(config.app().name()))
                .with_manifest_path(Some(config.app().manifest_path()))
                .with_target(Some(self.triple))
                .with_no_default_features(metadata.no_default_features())
                .with_args(metadata.cargo_args())
                .with_features(metadata.features())
        })
    }

    pub fn check(
        &self,
        config: &Config,
        metadata: &Metadata,
        env: &Env,
        noise_level: NoiseLevel,
    ) -> Result<(), CheckError> {
        self.cargo(config, metadata, "check")
            .map_err(CheckError::VersionCheckFailed)?
            .with_verbose(noise_level.pedantic())
            .build(env)
            .run()
            .map_err(CheckError::CargoCheckFailed)?;
        Ok(())
    }

    // NOTE: it's up to Xcode to pass the verbose flag here, so even when
    // using our build/run commands it won't get passed.
    // TODO: do something about that?
    #[allow(clippy::too_many_arguments)]
    pub fn compile_lib(
        &self,
        config: &Config,
        metadata: &Metadata,
        noise_level: NoiseLevel,
        force_color: bool,
        profile: Profile,
        env: &Env,
        cc_env: HashMap<&str, &OsStr>,
    ) -> Result<(), CompileLibError> {
        // Force color when running from CLI
        let color = if force_color { "always" } else { "auto" };
        self.cargo(config, metadata, "build")
            .map_err(CompileLibError::VersionCheckFailed)?
            .with_verbose(noise_level.pedantic())
            .with_release(profile.release())
            .build(env)
            .before_spawn(move |cmd| {
                cmd.args(["--color", color]);
                Ok(())
            })
            .vars(cc_env)
            .run()
            .map_err(CompileLibError::CargoBuildFailed)?;
        Ok(())
    }

    fn validate_sdk(&self) -> Result<(), SdkError> {
        if self.sdk == "iphonesimulator" {
            let xcode_developer_path =
                xcode_developer_dir().map_err(|e| SdkError::XcodePath(e.to_string()))?;
            let sdk_settings_path = xcode_developer_path.join(
                "Platforms/iPhoneSimulator.platform/Developer/SDKs/iPhoneSimulator.sdk/SDKSettings.plist",
            );
            // get Xcode's default SDK version
            let sdk_settings: plist::Value =
                plist::from_file(&sdk_settings_path).map_err(SdkError::ParseSdkSettings)?;
            let Some(version) = sdk_settings
                .as_dictionary()
                .and_then(|settings| settings.get("Version").and_then(|v| v.as_string()))
            else {
                return Err(SdkError::MissingSdkVersion);
            };

            // list installed runtimes
            let available_runtimes_output =
                duct::cmd("xcrun", ["simctl", "list", "runtimes", "--json"])
                    .stdout_capture()
                    .stderr_capture()
                    .run()
                    .map_err(|error| {
                        SdkError::Io {
                context:
                    "failed to list installed runtimes with `xcrun simctl list runtimes --json`",
                error,
            }
                    })?;
            let available_runtimes = serde_json::from_reader::<_, SimctlRuntimeList>(Cursor::new(
                available_runtimes_output.stdout,
            ))
            .map_err(SdkError::ParseRuntimes)?;

            // default SDK must be installed
            if !available_runtimes.runtimes.iter().any(|runtime| {
                (runtime.version == version || *runtime.version >= *version) && runtime.is_available
            }) {
                log::debug!(
                    "installed runtimes: {}",
                    available_runtimes
                        .runtimes
                        .into_iter()
                        .filter_map(|runtime| runtime.is_available.then_some(runtime.version))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                return Err(SdkError::SdkNotInstalled {
                    version: version.to_string(),
                });
            }
        }

        Ok(())
    }

    pub fn build(
        &self,
        target_device: Option<&Device>,
        config: &Config,
        env: &Env,
        _noise_level: opts::NoiseLevel,
        profile: opts::Profile,
        build_config: BuildConfig,
    ) -> Result<(), BuildError> {
        self.validate_sdk()?;
        let configuration = profile.as_str();
        let scheme = config.scheme();
        let workspace_path = config.workspace_path();
        let sdk = self.sdk.to_string();
        let arch = if self.is_macos() {
            Some(self.arch.to_string())
        } else {
            None
        };

        let destination = target_device
            .map(|device| format!("id={}", device.id()))
            .or_else(|| {
                if cfg!(target_arch = "x86_64") && self.sdk == "iphonesimulator" {
                    let destinations = list_destinations(&workspace_path, &scheme).unwrap();
                    let destination = destinations
                        .iter()
                        .filter(|d| d.platform == Some("iOS Simulator".to_string()))
                        .max_by_key(|d| d.os.as_deref().unwrap_or(""));
                    // on Intel we must force the ARCHS and destination when targeting the simulator
                    // otherwise xcodebuild tries to build arm64
                    Some(format!(
                        "platform=iOS Simulator,name={}",
                        destination
                            .and_then(|d| d.name.as_deref())
                            .unwrap_or("iPhone 13")
                    ))
                } else {
                    None
                }
            });

        let args: Vec<OsString> = vec![];
        duct::cmd("xcodebuild", args)
            .full_env(env.explicit_env())
            .env("FORCE_COLOR", "--force-color")
            .before_spawn(move |cmd| {
                build_config.xcodebuild_options.args_for(cmd);

                if let Some(a) = &arch {
                    cmd.args(["-arch", a]);
                }

                if let Some(destination) = &destination {
                    cmd.args(["-destination", destination]);
                }

                if cfg!(target_arch = "x86_64") && sdk == "iphonesimulator" {
                    // on Intel we must force the ARCHS when targeting the simulator
                    // otherwise xcodebuild tries to build arm64
                    cmd.arg("ARCHS=x86_64");
                }

                cmd.args(["-scheme", &scheme])
                    .arg("-workspace")
                    .arg(&workspace_path)
                    .args(["-sdk", &sdk])
                    .args(["-configuration", configuration])
                    .arg("build");
                Ok(())
            })
            .dup_stdio()
            .start()
            .map_err(|error| BuildError::Io {
                context: "failed to execute xcodebuild",
                error,
            })?
            .wait()
            .map_err(|error| BuildError::Io {
                context: "failed to build with xcodebuild",
                error,
            })?;
        Ok(())
    }

    pub fn archive(
        &self,
        config: &Config,
        env: &Env,
        noise_level: opts::NoiseLevel,
        profile: opts::Profile,
        new_version: Option<String>,
        archive_config: ArchiveConfig,
    ) -> Result<(), ArchiveError> {
        self.validate_sdk()?;

        if let Some(version) = new_version {
            util::with_working_dir(config.project_dir(), || {
                duct::cmd(
                    "xcrun",
                    ["agvtool", "new-version", "-all", &version.to_string()],
                )
                .dup_stdio()
                .run()
            })
            .map_err(ArchiveError::SetVersionFailed)?;
        }

        let configuration = profile.as_str();
        let archive_path = config.archive_dir().join(config.scheme());
        let scheme = config.scheme();
        let workspace_path = config.workspace_path();
        let sdk = self.sdk.to_string();
        let arch = if self.is_macos() {
            Some(self.arch.to_string())
        } else {
            None
        };

        let args: Vec<OsString> = vec![];
        duct::cmd("xcodebuild", args)
            .full_env(env.explicit_env())
            .before_spawn(move |cmd| {
                archive_config.xcodebuild_options.args_for(cmd);

                if let Some(v) = verbosity(noise_level) {
                    cmd.arg(v);
                }
                if let Some(a) = &arch {
                    cmd.args(["-arch", a]);
                }

                if cfg!(target_arch = "x86_64") && sdk == "iphonesimulator" {
                    let destinations = list_destinations(&workspace_path, &scheme).unwrap();
                    let destination = destinations
                        .iter()
                        .filter(|d| d.platform == Some("iOS Simulator".to_string()))
                        .max_by_key(|d| d.os.as_deref().unwrap_or(""));
                    // on Intel we must force the ARCHS and destination when targeting the simulator
                    // otherwise xcodebuild tries to build arm64
                    cmd.args([
                        "-destination",
                        &format!(
                            "platform=iOS Simulator,name={}",
                            destination
                                .and_then(|d| d.name.as_deref())
                                .unwrap_or("iPhone 13")
                        ),
                    ])
                    .arg("ARCHS=x86_64");
                }

                cmd.args(["-scheme", &scheme])
                    .arg("-workspace")
                    .arg(&workspace_path)
                    .args(["-sdk", &sdk])
                    .args(["-configuration", configuration])
                    .arg("-allowProvisioningUpdates")
                    .arg("archive")
                    .arg("-archivePath")
                    .arg(&archive_path);
                Ok(())
            })
            .dup_stdio()
            .start()
            .map_err(|error| ArchiveError::ArchiveFailed { error })?
            .wait()
            .map_err(|error| ArchiveError::ArchiveFailed { error })?;

        Ok(())
    }

    pub fn export(
        &self,
        config: &Config,
        env: &Env,
        noise_level: opts::NoiseLevel,
        export_config: ExportConfig,
    ) -> Result<(), ExportError> {
        // Super fun discrepancy in expectation of `-archivePath` value
        let archive_path = config
            .archive_dir()
            .join(format!("{}.xcarchive", config.scheme()));
        let export_dir = config.export_dir();
        let export_plist_path = config.export_plist_path();

        let args: Vec<OsString> = vec![];
        duct::cmd("xcodebuild", args)
            .full_env(env.explicit_env())
            .before_spawn(move |cmd| {
                export_config.xcodebuild_options.args_for(cmd);

                if let Some(v) = verbosity(noise_level) {
                    cmd.arg(v);
                }
                cmd.arg("-exportArchive")
                    .arg("-archivePath")
                    .arg(&archive_path)
                    .arg("-exportOptionsPlist")
                    .arg(&export_plist_path)
                    .arg("-exportPath")
                    .arg(&export_dir);

                Ok(())
            })
            .dup_stdio()
            .start()?
            .wait()?;

        Ok(())
    }
}

#[derive(Deserialize)]
struct SimctlRuntimeList {
    runtimes: Vec<SimctlRuntime>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SimctlRuntime {
    is_available: bool,
    version: String,
}
