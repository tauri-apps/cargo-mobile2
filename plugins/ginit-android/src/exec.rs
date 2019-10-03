use crate::{
    adb,
    config::Config,
    device::{Device, RunError, StacktraceError},
    env::{Env, Error as EnvError},
    target::{BuildError, CompileLibError, Target},
};
use ginit_core::{
    cli::{ArgInput, CliInput},
    opts::{NoiseLevel, Profile},
    target::{call_for_targets, call_for_targets_with_fallback, TargetInvalid},
    util::prompt,
};
use std::{fmt, io};

#[derive(Debug)]
pub enum Error {
    EnvInitFailed(EnvError),
    CommandInvalid(String),
    DeviceDetectionFailed(adb::DeviceListError),
    DevicePromptFailed(io::Error),
    NoDevicesDetected,
    TargetInvalid(TargetInvalid),
    CheckFailed(CompileLibError),
    BuildFailed(BuildError),
    RunFailed(RunError),
    StacktraceFailed(StacktraceError),
    ListFailed(adb::DeviceListError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::EnvInitFailed(err) => write!(f, "{}", err),
            Error::CommandInvalid(command) => write!(f, "Invalid command: {:?}", command),
            Error::DeviceDetectionFailed(err) => {
                write!(f, "Failed to detect connected Android devices: {}", err)
            }
            Error::DevicePromptFailed(err) => write!(f, "Failed to prompt for device: {}", err),
            Error::NoDevicesDetected => write!(f, "No connected Android devices detected."),
            Error::TargetInvalid(err) => write!(f, "Specified target was invalid: {}", err),
            Error::CheckFailed(err) => write!(f, "{}", err),
            Error::BuildFailed(err) => write!(f, "{}", err),
            Error::RunFailed(err) => write!(f, "{}", err),
            Error::StacktraceFailed(err) => write!(f, "{}", err),
            Error::ListFailed(err) => write!(f, "{}", err),
        }
    }
}

pub fn exec(config: &Config, input: CliInput, noise_level: NoiseLevel) -> Result<(), Error> {
    let env = Env::new().map_err(Error::EnvInitFailed)?;
    match input.command.as_str() {
        "check" => {
            let targets = input.targets().unwrap();
            call_for_targets(
                targets.iter(),
                // &detect_target_ok,
                // &env,
                |target: &Target| {
                    target
                        .check(config, &env, noise_level)
                        .map_err(Error::CheckFailed)
                },
            )
        }
        .map_err(Error::TargetInvalid)?,
        // "build" => call_for_targets_with_fallback(
        //     targets.iter(),
        //     &detect_target_ok,
        //     &env,
        //     |target: &Target| {
        //         target
        //             .build(config, &env, noise_level, profile)
        //             .map_err(Error::BuildFailed)
        //     },
        // )
        // .map_err(Error::TargetInvalid)?,
        // "run" => detect_device(&env)?
        //     .run(config, &env, noise_level, profile)
        //     .map_err(Error::RunFailed),
        // "st" => detect_device(&env)?
        //     .stacktrace(config, &env)
        //     .map_err(Error::StacktraceFailed),
        "list" => adb::device_list(&env)
            .map_err(Error::ListFailed)
            .map(|device_list| {
                prompt::list_display_only(device_list.iter(), device_list.len());
            }),
        _ => Err(Error::CommandInvalid(input.command)),
    }
}
