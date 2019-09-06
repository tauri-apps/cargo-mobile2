use crate::{
    config::Config,
    env::Env,
    init::cargo::CargoTarget,
    ios::system_profile::{self, DeveloperTools},
    opts::NoiseLevel,
    target::{Profile, TargetTrait},
    util::{self, pure_command::PureCommand},
};
use into_result::{command::CommandError, IntoResult as _};
use std::{collections::BTreeMap, fmt, path::Path, process::Command};

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
    CargoCheckFailed(CommandError),
}

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckError::VersionCheckFailed(err) => write!(f, "Xcode version check failed: {}", err),
            CheckError::CargoCheckFailed(err) => write!(f, "Failed to run `cargo check`: {}", err),
        }
    }
}

#[derive(Debug)]
pub enum CompileLibError {
    VersionCheckFailed(VersionCheckError),
    CargoBuildFailed(CommandError),
}

impl fmt::Display for CompileLibError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileLibError::VersionCheckFailed(err) => {
                write!(f, "Xcode version check failed: {}", err)
            }
            CompileLibError::CargoBuildFailed(err) => {
                write!(f, "Failed to run `cargo build`: {}", err)
            }
        }
    }
}

#[derive(Debug)]
pub struct BuildError(CommandError);

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to build via `xcodebuild`: {}", self.0)
    }
}

#[derive(Debug)]
pub enum ArchiveError {
    ArchiveFailed(CommandError),
    ExportFailed(CommandError),
}

impl fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchiveError::ArchiveFailed(err) => {
                write!(f, "Failed to archive via `xcodebuild: {}", err)
            }
            ArchiveError::ExportFailed(err) => {
                write!(f, "Failed to export archive via `xcodebuild: {}", err)
            }
        }
    }
}

#[derive(Debug)]
pub enum RunError {
    BuildFailed(BuildError),
    ArchiveFailed(ArchiveError),
    UnzipFailed(CommandError),
    IosDeployMissing(IosDeployMissing),
    DeployFailed(CommandError),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunError::BuildFailed(err) => write!(f, "Failed to build app: {}", err),
            RunError::ArchiveFailed(err) => write!(f, "Failed to archive app: {}", err),
            RunError::UnzipFailed(err) => write!(f, "Failed to unzip archive: {}", err),
            RunError::IosDeployMissing(err) => write!(f, "{}", err),
            RunError::DeployFailed(err) => {
                write!(f, "Failed to deploy app via `ios-deploy`: {}", err)
            }
        }
    }
}

#[derive(Debug)]
pub struct IosDeployMissing;

impl fmt::Display for IosDeployMissing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "`ios-deploy` not found. Please run `cargo {} init` and try again. If it still doesn't work after that, then this is a bug!",
            crate::NAME
        )
    }
}

fn ios_deploy(env: &Env) -> Result<Command, IosDeployMissing> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("ios-deploy/build/Release/ios-deploy");
    if path.exists() {
        Ok(PureCommand::new(path, env))
    } else {
        Err(IosDeployMissing)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Target<'a> {
    pub triple: &'a str,
    pub arch: &'a str,
    min_xcode_version: Option<((u32, u32), &'static str)>,
}

impl<'a> TargetTrait<'a> for Target<'a> {
    const DEFAULT_KEY: &'static str = "aarch64";

    fn all() -> &'a BTreeMap<&'a str, Self> {
        lazy_static::lazy_static! {
            pub static ref TARGETS: BTreeMap<&'static str, Target<'static>> = {
                let mut targets = BTreeMap::new();
                targets.insert("aarch64", Target {
                    triple: "aarch64-apple-ios",
                    arch: "arm64",
                    min_xcode_version: None,
                });
                targets.insert("x86_64", Target {
                    triple: "x86_64-apple-ios",
                    arch: "x86_64",
                    // Simulator only supports Metal as of Xcode 11.0:
                    // https://developer.apple.com/documentation/metal/developing_metal_apps_that_run_in_simulator?language=objc
                    // While this doesn't matter if you aren't using Metal,
                    // it should be fine to be opinionated about this given
                    // OpenGL's deprecation.
                    min_xcode_version: Some(((11, 0), "iOS Simulator doesn't support Metal until")),
                });
                targets
            };
        }
        &*TARGETS
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
            min_xcode_version: None,
        }
    }

    pub fn is_macos(&self) -> bool {
        *self == Self::macos()
    }

    pub fn generate_cargo_config(&self) -> CargoTarget {
        Default::default()
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
    ) -> Result<util::CargoCommand<'a>, VersionCheckError> {
        self.min_xcode_version_satisfied().map(|()| {
            util::CargoCommand::new(subcommand)
                .with_package(Some(config.app_name()))
                .with_manifest_path(Some(config.manifest_path()))
                .with_target(Some(&self.triple))
                .with_features(Some("metal"))
                .with_no_default_features(!self.is_macos())
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
            .with_verbose(noise_level.is_pedantic())
            .into_command(env)
            .status()
            .into_result()
            .map_err(CheckError::CargoCheckFailed)
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
            .with_verbose(noise_level.is_pedantic())
            .with_release(profile.is_release())
            .into_command_impure()
            .status()
            .into_result()
            .map_err(CompileLibError::CargoBuildFailed)
    }

    pub fn build(&self, config: &Config, env: &Env, profile: Profile) -> Result<(), BuildError> {
        let configuration = profile.as_str();
        PureCommand::new("xcodebuild", env)
            .args(&["-scheme", &config.ios().scheme()])
            .arg("-workspace")
            .arg(&config.ios().workspace_path())
            .args(&["-configuration", configuration])
            .args(&["-arch", self.arch])
            .arg("build")
            .status()
            .into_result()
            .map_err(BuildError)
    }

    fn archive(&self, config: &Config, env: &Env, profile: Profile) -> Result<(), ArchiveError> {
        let configuration = profile.as_str();
        let archive_path = config.ios().export_path().join(&config.ios().scheme());
        PureCommand::new("xcodebuild", env)
            .args(&["-scheme", &config.ios().scheme()])
            .arg("-workspace")
            .arg(&config.ios().workspace_path())
            .args(&["-sdk", "iphoneos"])
            .args(&["-configuration", configuration])
            .args(&["-arch", self.arch])
            .arg("archive")
            .arg("-archivePath")
            .arg(&archive_path)
            .status()
            .into_result()
            .map_err(ArchiveError::ArchiveFailed)?;
        // Super fun discrepancy in expectation of `-archivePath` value
        let archive_path = config
            .ios()
            .export_path()
            .join(&format!("{}.xcarchive", config.ios().scheme()));
        PureCommand::new("xcodebuild", env)
            .arg("-exportArchive")
            .arg("-archivePath")
            .arg(&archive_path)
            .arg("-exportOptionsPlist")
            .arg(&config.ios().export_plist_path())
            .arg("-exportPath")
            .arg(&config.ios().export_path())
            .status()
            .into_result()
            .map_err(ArchiveError::ExportFailed)
    }

    pub fn run(&self, config: &Config, env: &Env, profile: Profile) -> Result<(), RunError> {
        // TODO: These steps are run unconditionally, which is slooooooow
        self.build(config, env, profile)
            .map_err(RunError::BuildFailed)?;
        self.archive(config, env, profile)
            .map_err(RunError::ArchiveFailed)?;
        PureCommand::new("unzip", env)
            .arg("-o") // -o = always overwrite
            .arg(&config.ios().ipa_path())
            .arg("-d")
            .arg(&config.ios().export_path())
            .status()
            .into_result()
            .map_err(RunError::UnzipFailed)?;
        // This dies if the device is locked, and gives you no time to react to
        // that. `ios-deploy --detect` can apparently be used to check in
        // advance, giving us an opportunity to promt. Though, it's much more
        // relaxing to just turn off auto-lock under Display & Brightness.
        ios_deploy(env)
            .map_err(RunError::IosDeployMissing)?
            .arg("--debug")
            .arg("--bundle")
            .arg(&config.ios().app_path())
            // This tool can apparently install over wifi, but not debug over
            // wifi... so if your device is connected over wifi (even if it's
            // wired as well) and we're using the `--debug` flag, then
            // launching will fail unless we also specify the `--no-wifi` flag
            // to keep it from trying that.
            .arg("--no-wifi")
            .status()
            .into_result()
            .map_err(RunError::DeployFailed)
    }
}
