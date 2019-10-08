use crate::{
    config::Config,
    device::{Device, RunError},
    ios_deploy,
    target::{BuildError, CheckError, CompileLibError, Target},
};
use ginit_core::{
    cli::CliInput,
    define_device_prompt,
    device::PromptError,
    env::{Env, Error as EnvError},
    opts::NoiseLevel,
    target::{call_for_targets_with_fallback, TargetInvalid},
    util::prompt,
};
use std::fmt::{self, Display};

#[derive(Debug)]
pub enum Error {
    CommandInvalid(String),
    EnvInitFailed(EnvError),
    DevicePromptFailed(PromptError<ios_deploy::DeviceListError>),
    TargetInvalid(TargetInvalid),
    CheckFailed(CheckError),
    BuildFailed(BuildError),
    RunFailed(RunError),
    ListFailed(ios_deploy::DeviceListError),
    ArchInvalid { arch: String },
    CompileLibFailed(CompileLibError),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CommandInvalid(command) => write!(f, "Invalid command: {:?}", command),
            Self::EnvInitFailed(err) => write!(f, "{}", err),
            Self::DevicePromptFailed(err) => write!(f, "{}", err),
            Self::TargetInvalid(err) => write!(f, "Specified target was invalid: {}", err),
            Self::CheckFailed(err) => write!(f, "{}", err),
            Self::BuildFailed(err) => write!(f, "{}", err),
            Self::RunFailed(err) => write!(f, "{}", err),
            Self::ListFailed(err) => write!(f, "{}", err),
            Self::ArchInvalid { arch } => write!(f, "Specified arch was invalid: {}", arch),
            Self::CompileLibFailed(err) => write!(f, "{}", err),
        }
    }
}

pub fn exec(config: &Config, input: CliInput, noise_level: NoiseLevel) -> Result<(), Error> {
    define_device_prompt!(ios_deploy::device_list, ios_deploy::DeviceListError, iOS);
    fn detect_target_ok<'a>(env: &Env) -> Option<&'a Target<'a>> {
        device_prompt(env).map(|device| device.target()).ok()
    }

    let env = Env::new().map_err(Error::EnvInitFailed)?;
    match input.command.as_str() {
        "check" => {
            let targets = input.targets().unwrap();
            call_for_targets_with_fallback(
                targets.iter(),
                &detect_target_ok,
                &env,
                |target: &Target| {
                    target
                        .check(config, &env, noise_level)
                        .map_err(Error::CheckFailed)
                },
            )
        }
        .map_err(Error::TargetInvalid)?,
        "build" => {
            let targets = input.targets().unwrap();
            let profile = input.profile().unwrap();
            call_for_targets_with_fallback(
                targets.iter(),
                &detect_target_ok,
                &env,
                |target: &Target| {
                    target
                        .build(config, &env, profile)
                        .map_err(Error::BuildFailed)
                },
            )
        }
        .map_err(Error::TargetInvalid)?,
        "run" => {
            let profile = input.profile().unwrap();
            device_prompt(&env)
                .map_err(Error::DevicePromptFailed)?
                .run(config, &env, profile)
                .map_err(Error::RunFailed)
        }
        "list" => ios_deploy::device_list(&env)
            .map_err(Error::ListFailed)
            .map(|device_list| {
                prompt::list_display_only(device_list.iter(), device_list.len());
            }),
        "compile-lib" => {
            let macos = input.get_named_presence("macos");
            let arch = input.get_named_value("ARCH").unwrap();
            let profile = input.profile().unwrap();
            match macos {
                true => Target::macos().compile_lib(config, noise_level, profile),
                false => Target::for_arch(&arch)
                    .ok_or_else(|| Error::ArchInvalid {
                        arch: arch.to_owned(),
                    })?
                    .compile_lib(config, noise_level, profile),
            }
        }
        .map_err(Error::CompileLibFailed),
        _ => Err(Error::CommandInvalid(input.command)),
    }
}
