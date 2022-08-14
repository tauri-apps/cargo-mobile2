use super::{
    config::{Config, Metadata},
    system_profile::{self, DeveloperTools},
    version_number::VersionNumber,
};
use crate::{
    env::{Env, ExplicitEnv as _},
    opts::{self, ForceColor, NoiseLevel, Profile},
    target::TargetTrait,
    util::{
        self,
        cli::{Report, Reportable},
        CargoCommand, WithWorkingDirError,
    },
};
use once_cell_regex::exports::once_cell::sync::OnceCell;
use std::{
    collections::{BTreeMap, HashMap},
    ffi::OsStr,
};

fn verbosity(noise_level: opts::NoiseLevel) -> Option<&'static str> {
    if noise_level.pedantic() {
        None
    } else {
        Some("-quiet")
    }
}

#[derive(Debug)]
pub enum VersionCheckError {
    LookupFailed(system_profile::Error),
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
    CargoCheckFailed(bossy::Error),
}

impl Reportable for CheckError {
    fn report(&self) -> Report {
        match self {
            Self::VersionCheckFailed(err) => err.report(),
            Self::CargoCheckFailed(err) => Report::error("Failed to run `cargo check`", err),
        }
    }
}

#[derive(Debug)]
pub enum CompileLibError {
    VersionCheckFailed(VersionCheckError),
    CargoBuildFailed(bossy::Error),
}

impl Reportable for CompileLibError {
    fn report(&self) -> Report {
        match self {
            Self::VersionCheckFailed(err) => err.report(),
            Self::CargoBuildFailed(err) => Report::error("Failed to run `cargo build`", err),
        }
    }
}

#[derive(Debug)]
pub struct BuildError(bossy::Error);

impl Reportable for BuildError {
    fn report(&self) -> Report {
        Report::error("Failed to build via `xcodebuild`", &self.0)
    }
}

#[derive(Debug)]
pub enum ArchiveError {
    SetVersionFailed(WithWorkingDirError<bossy::Error>),
    ArchiveFailed(bossy::Error),
}

impl Reportable for ArchiveError {
    fn report(&self) -> Report {
        match self {
            Self::SetVersionFailed(err) => Report::error("Failed to set app version number", err),
            Self::ArchiveFailed(err) => Report::error("Failed to archive via `xcodebuild`", err),
        }
    }
}

#[derive(Debug)]
pub struct ExportError(bossy::Error);

impl Reportable for ExportError {
    fn report(&self) -> Report {
        Report::error("Failed to export archive via `xcodebuild`", &self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Target<'a> {
    pub triple: &'a str,
    pub arch: &'a str,
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
                    alias: Some("arm64e"),
                    min_xcode_version: None,
                },
            );
            targets.insert(
                "x86_64",
                Target {
                    triple: "x86_64-apple-ios",
                    arch: "x86_64",
                    alias: None,
                    // Simulator only supports Metal as of Xcode 11.0:
                    // https://developer.apple.com/documentation/metal/developing_metal_apps_that_run_in_simulator?language=objc
                    // While this doesn't matter if you aren't using Metal,
                    // it should be fine to be opinionated about this given
                    // OpenGL's deprecation.
                    min_xcode_version: Some(((11, 0), "iOS Simulator doesn't support Metal until")),
                },
            );
            targets
        })
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
                .with_target(Some(&self.triple))
                .with_no_default_features(metadata.no_default_features())
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
            .into_command_pure(env)
            .run_and_wait()
            .map_err(CheckError::CargoCheckFailed)?;
        Ok(())
    }

    // NOTE: it's up to Xcode to pass the verbose flag here, so even when
    // using our build/run commands it won't get passed.
    // TODO: do something about that?
    pub fn compile_lib(
        &self,
        config: &Config,
        metadata: &Metadata,
        noise_level: NoiseLevel,
        force_color: ForceColor,
        profile: Profile,
        env: &Env,
        cc_env: HashMap<&str, &OsStr>,
    ) -> Result<(), CompileLibError> {
        // Force color when running from CLI
        let color = if force_color.yes() { "always" } else { "auto" };
        self.cargo(config, metadata, "build")
            .map_err(CompileLibError::VersionCheckFailed)?
            .with_verbose(noise_level.pedantic())
            .with_release(profile.release())
            .into_command_pure(env)
            .with_env_vars(cc_env)
            .with_args(&["--color", color])
            .run_and_wait()
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
        bossy::Command::pure("xcodebuild")
            .with_env_vars(env.explicit_env())
            .with_env_var("FORCE_COLOR", "--force-color")
            .with_args(verbosity(noise_level))
            .with_args(&["-scheme", &config.scheme()])
            .with_arg("-workspace")
            .with_arg(&config.workspace_path())
            .with_args(&["-configuration", configuration])
            .with_args(&["-arch", self.arch])
            .with_arg("-allowProvisioningUpdates")
            .with_arg("build")
            .run_and_wait()
            .map_err(BuildError)?;
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
                bossy::Command::pure_parse("xcrun agvtool new-version -all")
                    .with_arg(&build_number.to_string())
                    .run_and_wait()
            })
            .map_err(ArchiveError::SetVersionFailed)?;
        }
        let configuration = profile.as_str();
        let archive_path = config.archive_dir().join(&config.scheme());
        bossy::Command::pure("xcodebuild")
            .with_env_vars(env.explicit_env())
            .with_args(verbosity(noise_level))
            .with_args(&["-scheme", &config.scheme()])
            .with_arg("-workspace")
            .with_arg(&config.workspace_path())
            .with_args(&["-sdk", "iphoneos"])
            .with_args(&["-configuration", configuration])
            .with_args(&["-arch", self.arch])
            .with_arg("-allowProvisioningUpdates")
            .with_arg("archive")
            .with_arg("-archivePath")
            .with_arg(&archive_path)
            .run_and_wait()
            .map_err(ArchiveError::ArchiveFailed)?;
        Ok(())
    }

    pub fn export(
        &self,
        config: &Config,
        env: &Env,
        noise_level: opts::NoiseLevel,
    ) -> Result<(), ExportError> {
        // Super fun discrepancy in expectation of `-archivePath` value
        let archive_path = config
            .archive_dir()
            .join(&format!("{}.xcarchive", config.scheme()));
        bossy::Command::pure("xcodebuild")
            .with_env_vars(env.explicit_env())
            .with_args(verbosity(noise_level))
            .with_arg("-exportArchive")
            .with_arg("-archivePath")
            .with_arg(&archive_path)
            .with_arg("-exportOptionsPlist")
            .with_arg(&config.export_plist_path())
            .with_arg("-exportPath")
            .with_arg(&config.export_dir())
            .run_and_wait()
            .map_err(ExportError)?;
        Ok(())
    }
}
