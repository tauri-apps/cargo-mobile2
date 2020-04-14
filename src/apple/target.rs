use super::{
    config::Config,
    system_profile::{self, DeveloperTools},
};
use crate::{
    env::{Env, ExplicitEnv as _},
    opts::{NoiseLevel, Profile},
    target::TargetTrait,
    util::CargoCommand,
};
use once_cell_regex::exports::once_cell::sync::OnceCell;
use std::{
    collections::BTreeMap,
    fmt::{self, Display},
};

#[derive(Debug)]
pub enum VersionCheckError {
    LookupFailed(system_profile::Error),
    TooLow {
        msg: &'static str,
        you_have: (u32, u32),
        you_need: (u32, u32),
    },
}

impl fmt::Display for VersionCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionCheckError::LookupFailed(err) => {
                write!(f, "Failed to lookup Xcode version: {}", err)
            }
            VersionCheckError::TooLow {
                msg,
                you_have,
                you_need,
            } => write!(
                f,
                "{} Xcode {}.{}; you have Xcode {}.{}.",
                msg, you_need.0, you_need.1, you_have.0, you_have.1
            ),
        }
    }
}

#[derive(Debug)]
pub enum CheckError {
    VersionCheckFailed(VersionCheckError),
    CargoCheckFailed(bossy::Error),
}

impl Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VersionCheckFailed(err) => write!(f, "Xcode version check failed: {}", err),
            Self::CargoCheckFailed(err) => write!(f, "Failed to run `cargo check`: {}", err),
        }
    }
}

#[derive(Debug)]
pub enum CompileLibError {
    VersionCheckFailed(VersionCheckError),
    CargoBuildFailed(bossy::Error),
}

impl Display for CompileLibError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VersionCheckFailed(err) => write!(f, "Xcode version check failed: {}", err),
            Self::CargoBuildFailed(err) => write!(f, "Failed to run `cargo build`: {}", err),
        }
    }
}

#[derive(Debug)]
pub struct BuildError(bossy::Error);

impl Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to build via `xcodebuild`: {}", self.0)
    }
}

#[derive(Debug)]
pub enum ArchiveError {
    ArchiveFailed(bossy::Error),
    ExportFailed(bossy::Error),
}

impl Display for ArchiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArchiveFailed(err) => write!(f, "Failed to archive via `xcodebuild: {}", err),
            Self::ExportFailed(err) => {
                write!(f, "Failed to export archive via `xcodebuild: {}", err)
            }
        }
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
        subcommand: &'a str,
    ) -> Result<CargoCommand<'a>, VersionCheckError> {
        let (no_default_features, features) = if self.is_macos() {
            (config.macos_no_default_features(), config.macos_features())
        } else {
            (config.ios_no_default_features(), config.ios_features())
        };
        self.min_xcode_version_satisfied().map(|()| {
            CargoCommand::new(subcommand)
                .with_package(Some(config.app().name()))
                .with_manifest_path(Some(config.app().manifest_path()))
                .with_target(Some(&self.triple))
                .with_no_default_features(no_default_features)
                .with_features(Some(features))
        })
    }

    pub fn check(
        &self,
        config: &Config,
        env: &Env,
        noise_level: NoiseLevel,
    ) -> Result<(), CheckError> {
        self.cargo(config, "check")
            .map_err(CheckError::VersionCheckFailed)?
            .with_verbose(noise_level.pedantic())
            .into_command_pure(env)
            .run_and_wait()
            .map_err(CheckError::CargoCheckFailed)?;
        Ok(())
    }

    pub fn compile_lib(
        &self,
        config: &Config,
        noise_level: NoiseLevel,
        profile: Profile,
    ) -> Result<(), CompileLibError> {
        // NOTE: it's up to Xcode to pass the verbose flag here, so even when
        // using our build/run commands it won't get passed.
        // TODO: I don't undestand this comment
        self.cargo(config, "build")
            .map_err(CompileLibError::VersionCheckFailed)?
            .with_verbose(noise_level.pedantic())
            .with_release(profile.release())
            .into_command_impure()
            .run_and_wait()
            .map_err(CompileLibError::CargoBuildFailed)?;
        Ok(())
    }

    pub fn build(&self, config: &Config, env: &Env, profile: Profile) -> Result<(), BuildError> {
        let configuration = profile.as_str();
        bossy::Command::pure("xcodebuild")
            .with_env_vars(env.explicit_env())
            .with_args(&["-scheme", &config.scheme()])
            .with_arg("-workspace")
            .with_arg(&config.workspace_path())
            .with_args(&["-configuration", configuration])
            .with_args(&["-arch", self.arch])
            .with_arg("build")
            .run_and_wait()
            .map_err(BuildError)?;
        Ok(())
    }

    pub(super) fn archive(
        &self,
        config: &Config,
        env: &Env,
        profile: Profile,
    ) -> Result<(), ArchiveError> {
        let configuration = profile.as_str();
        let archive_path = config.export_dir().join(&config.scheme());
        bossy::Command::pure("xcodebuild")
            .with_env_vars(env.explicit_env())
            .with_args(&["-scheme", &config.scheme()])
            .with_arg("-workspace")
            .with_arg(&config.workspace_path())
            .with_args(&["-sdk", "iphoneos"])
            .with_args(&["-configuration", configuration])
            .with_args(&["-arch", self.arch])
            .with_arg("archive")
            .with_arg("-archivePath")
            .with_arg(&archive_path)
            .run_and_wait()
            .map_err(ArchiveError::ArchiveFailed)?;
        // Super fun discrepancy in expectation of `-archivePath` value
        let archive_path = config
            .export_dir()
            .join(&format!("{}.xcarchive", config.scheme()));
        bossy::Command::pure("xcodebuild")
            .with_env_vars(env.explicit_env())
            .with_arg("-exportArchive")
            .with_arg("-archivePath")
            .with_arg(&archive_path)
            .with_arg("-exportOptionsPlist")
            .with_arg(&config.export_plist_path())
            .with_arg("-exportPath")
            .with_arg(&config.export_dir())
            .run_and_wait()
            .map_err(ArchiveError::ExportFailed)?;
        Ok(())
    }
}
