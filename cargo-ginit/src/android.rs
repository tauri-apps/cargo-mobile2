use crate::util::{parse_profile, parse_targets, take_a_target_list};
use clap::{App, AppSettings, ArgMatches, SubCommand};
use ginit::{
    android::{
        adb,
        device::{Device, RunError, StacktraceError},
        env::{Env, Error as EnvError},
        target::{BuildError, CompileLibError, Target},
    },
    config::Config,
    opts::{NoiseLevel, Profile},
    target::{call_for_targets_with_fallback, TargetInvalid},
};
use std::fmt;

pub fn subcommand<'a, 'b>(targets: &'a [&'a str]) -> App<'a, 'b> {
    SubCommand::with_name("android")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .about("Tools for Android")
        .subcommand(
            SubCommand::with_name("check")
                .about("Checks if code compiles for target(s)")
                .display_order(0)
                .arg(take_a_target_list::<Target>(targets)),
        )
        .subcommand(
            SubCommand::with_name("build")
                .about("Builds dynamic libraries for target(s)")
                .display_order(1)
                .arg(take_a_target_list::<Target>(targets))
                .arg_from_usage("--release 'Build with release optimizations'"),
        )
        .subcommand(
            SubCommand::with_name("run")
                .about("Deploys APK for target")
                .display_order(2)
                .arg_from_usage("--release 'Build with release optimizations'"),
        )
        .subcommand(
            SubCommand::with_name("st")
                .display_order(3)
                .about("Displays a detailed stacktrace for a target"),
        )
        .subcommand(
            SubCommand::with_name("list")
                .about("Lists connected devices")
                .display_order(4),
        )
}

#[derive(Debug)]
pub enum Error {
    EnvInitFailed(EnvError),
    DeviceDetectionFailed(adb::DeviceListError),
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
            Error::DeviceDetectionFailed(err) => {
                write!(f, "Failed to detect connected Android devices: {}", err)
            }
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

#[derive(Debug)]
pub enum AndroidCommand {
    Check {
        targets: Vec<String>,
    },
    Build {
        targets: Vec<String>,
        profile: Profile,
    },
    Run {
        profile: Profile,
    },
    Stacktrace,
    List,
}

impl AndroidCommand {
    pub fn parse(matches: ArgMatches<'_>) -> Self {
        let subcommand = matches.subcommand.as_ref().unwrap(); // clap makes sure we got a subcommand
        match subcommand.name.as_str() {
            "check" => AndroidCommand::Check {
                targets: parse_targets(&subcommand.matches),
            },
            "build" => AndroidCommand::Build {
                targets: parse_targets(&subcommand.matches),
                profile: parse_profile(&subcommand.matches),
            },
            "run" => AndroidCommand::Run {
                profile: parse_profile(&subcommand.matches),
            },
            "st" => AndroidCommand::Stacktrace,
            "list" => AndroidCommand::List,
            _ => unreachable!(), // clap will reject anything else
        }
    }

    pub fn exec(self, config: &Config, noise_level: NoiseLevel) -> Result<(), Error> {
        fn detect_device<'a>(env: &Env) -> Result<Device<'a>, Error> {
            let device_list = adb::device_list(env).map_err(Error::DeviceDetectionFailed)?;
            if device_list.len() > 0 {
                // By default, we're just taking the first device, which isn't super exciting.
                let device = device_list.into_iter().next().unwrap();
                println!(
                    "Detected connected device: {} with target {:?}",
                    device,
                    device.target().triple,
                );
                Ok(device)
            } else {
                Err(Error::NoDevicesDetected)
            }
        }

        fn detect_target_ok<'a>(env: &Env) -> Option<&'a Target<'a>> {
            detect_device(env).map(|device| device.target()).ok()
        }

        let env = Env::new().map_err(Error::EnvInitFailed)?;
        match self {
            AndroidCommand::Check { targets } => call_for_targets_with_fallback(
                targets.iter(),
                &detect_target_ok,
                &env,
                |target: &Target| {
                    target
                        .check(config, &env, noise_level)
                        .map_err(Error::CheckFailed)
                },
            )
            .map_err(Error::TargetInvalid)?,
            AndroidCommand::Build { targets, profile } => call_for_targets_with_fallback(
                targets.iter(),
                &detect_target_ok,
                &env,
                |target: &Target| {
                    target
                        .build(config, &env, noise_level, profile)
                        .map_err(Error::BuildFailed)
                },
            )
            .map_err(Error::TargetInvalid)?,
            AndroidCommand::Run { profile } => detect_device(&env)?
                .run(config, &env, noise_level, profile)
                .map_err(Error::RunFailed),
            AndroidCommand::Stacktrace => detect_device(&env)?
                .stacktrace(config, &env)
                .map_err(Error::StacktraceFailed),
            AndroidCommand::List => adb::device_list(&env)
                .map(|device_list| {
                    for (index, device) in device_list.iter().enumerate() {
                        println!("  [{}] {}", index, device);
                    }
                })
                .map_err(Error::ListFailed),
        }
    }
}
