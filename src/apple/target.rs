use super::{
    config::{Config, Metadata},
    system_profile::{self, DeveloperTools},
    version_number::VersionNumber,
};
use crate::{
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
use std::{
    collections::{BTreeMap, HashMap},
    ffi::{OsStr, OsString},
    path::PathBuf,
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
#[error(transparent)]
pub struct BuildError(#[from] std::io::Error);

impl Reportable for BuildError {
    fn report(&self) -> Report {
        Report::error("Failed to build via `xcodebuild`", &self.0)
    }
}

#[derive(Debug, Error)]
pub enum ArchiveError {
    #[error("Failed to set app version number: {0}")]
    SetVersionFailed(WithWorkingDirError<std::io::Error>),
    #[error("Failed to archive via `xcodebuild`: {0}")]
    ArchiveFailed(#[from] std::io::Error),
}

impl Reportable for ArchiveError {
    fn report(&self) -> Report {
        match self {
            Self::SetVersionFailed(err) => Report::error("Failed to set app version number", err),
            Self::ArchiveFailed(err) => Report::error("Failed to archive via `xcodebuild`", err),
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
pub struct ExportConfig {
    allow_provisioning_updates: bool,
    authentication_credentials: Option<AuthCredentials>,
}

impl ExportConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn allow_provisioning_updates(mut self) -> Self {
        self.allow_provisioning_updates = true;
        self
    }

    pub fn authentication_credentials(mut self, credentials: AuthCredentials) -> Self {
        self.authentication_credentials.replace(credentials);
        self
    }
}

pub struct AuthCredentials {
    pub key_path: PathBuf,
    pub key_id: String,
    pub key_issuer_id: String,
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
            sdk: "iphoneos",
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

    pub fn build(
        &self,
        config: &Config,
        env: &Env,
        noise_level: opts::NoiseLevel,
        profile: opts::Profile,
    ) -> Result<(), BuildError> {
        let configuration = profile.as_str();
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
            .env("FORCE_COLOR", "--force-color")
            .before_spawn(move |cmd| {
                if let Some(v) = verbosity(noise_level) {
                    cmd.arg(v);
                }
                if let Some(a) = &arch {
                    cmd.args(["-arch", a]);
                }
                cmd.args(["-scheme", &scheme])
                    .arg("-workspace")
                    .arg(&workspace_path)
                    .args(["-sdk", &sdk])
                    .args(["-configuration", configuration])
                    .arg("-allowProvisioningUpdates")
                    .arg("build");
                Ok(())
            })
            .dup_stdio()
            .start()?
            .wait()?;
        Ok(())
    }

    pub fn archive(
        &self,
        config: &Config,
        env: &Env,
        noise_level: opts::NoiseLevel,
        profile: opts::Profile,
        build_number: Option<VersionNumber>,
    ) -> Result<(), ArchiveError> {
        if let Some(build_number) = build_number {
            util::with_working_dir(config.project_dir(), || {
                duct::cmd(
                    "xcrun",
                    ["agvtool", "new-version", "-all", &build_number.to_string()],
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
                if let Some(v) = verbosity(noise_level) {
                    cmd.arg(v);
                }
                if let Some(a) = &arch {
                    cmd.args(["-arch", a]);
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
            .start()?
            .wait()?;

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

                if export_config.allow_provisioning_updates {
                    cmd.arg("-allowProvisioningUpdates");
                }
                if let Some(credentials) = &export_config.authentication_credentials {
                    cmd.args(["-authenticationKeyID", &credentials.key_id])
                        .arg("-authenticationKeyPath")
                        .arg(&credentials.key_path)
                        .args(["-authenticationKeyIssuerID", &credentials.key_issuer_id]);
                }

                Ok(())
            })
            .dup_stdio()
            .start()?
            .wait()?;

        Ok(())
    }
}
