use crate::util::{parse_profile, parse_targets, take_a_target_list};
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use ginit::{
    config::Config,
    env::{Env, Error as EnvError},
    ios::{
        device::{Device, RunError},
        ios_deploy,
        target::{BuildError, CheckError, CompileLibError, Target},
    },
    opts::{NoiseLevel, Profile},
    target::{call_for_targets_with_fallback, TargetInvalid},
    util::prompt,
};
use std::{fmt, io};

pub fn subcommand<'a, 'b>(targets: &'a [&'a str]) -> App<'a, 'b> {
    SubCommand::with_name("ios")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .about("Tools for iOS")
        .subcommand(
            SubCommand::with_name("check")
                .about("Checks if code compiles for target(s)")
                .display_order(0)
                .arg(take_a_target_list::<Target>(targets)),
        )
        .subcommand(
            SubCommand::with_name("build")
                .about("Builds static library")
                .display_order(1)
                .arg(take_a_target_list::<Target>(targets))
                .arg_from_usage("--release 'Build with release optimizations'"),
        )
        .subcommand(
            SubCommand::with_name("run")
                .about("Deploys IPA to connected device")
                .display_order(2)
                .arg_from_usage("--release 'Build with release optimizations'"),
        )
        .subcommand(
            SubCommand::with_name("list")
                .about("Lists connected devices")
                .display_order(3),
        )
        .subcommand(
            SubCommand::with_name("compile-lib")
                .setting(AppSettings::Hidden)
                .about("Compiles static lib (should only be called by Xcode!)")
                .arg_from_usage("--macos 'Awkwardly special-case for macOS'")
                .arg(Arg::with_name("ARCH").index(1).required(true))
                .arg_from_usage("--release 'Build with release optimizations'"),
        )
}

#[derive(Debug)]
pub enum Error {
    EnvInitFailed(EnvError),
    DeviceDetectionFailed(ios_deploy::DeviceListError),
    DevicePromptFailed(io::Error),
    NoDevicesDetected,
    TargetInvalid(TargetInvalid),
    CheckFailed(CheckError),
    BuildFailed(BuildError),
    RunFailed(RunError),
    ListFailed(ios_deploy::DeviceListError),
    ArchInvalid { arch: String },
    CompileLibFailed(CompileLibError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::EnvInitFailed(err) => write!(f, "{}", err),
            Error::DeviceDetectionFailed(err) => {
                write!(f, "Failed to detect connected iOS devices: {}", err)
            }
            Error::DevicePromptFailed(err) => write!(f, "Failed to prompt for device: {}", err),
            Error::NoDevicesDetected => write!(f, "No connected iOS devices detected."),
            Error::TargetInvalid(err) => write!(f, "Specified target was invalid: {}", err),
            Error::CheckFailed(err) => write!(f, "{}", err),
            Error::BuildFailed(err) => write!(f, "{}", err),
            Error::RunFailed(err) => write!(f, "{}", err),
            Error::ListFailed(err) => write!(f, "{}", err),
            Error::ArchInvalid { arch } => write!(f, "Specified arch was invalid: {}", arch),
            Error::CompileLibFailed(err) => write!(f, "{}", err),
        }
    }
}

#[derive(Debug)]
pub enum IosCommand {
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
    List,
    CompileLib {
        macos: bool,
        arch: String,
        profile: Profile,
    },
}

impl IosCommand {
    pub fn parse(matches: ArgMatches<'_>) -> Self {
        let subcommand = matches.subcommand.as_ref().unwrap(); // clap makes sure we got a subcommand
        match subcommand.name.as_str() {
            "check" => IosCommand::Check {
                targets: parse_targets(&subcommand.matches),
            },
            "build" => IosCommand::Build {
                targets: parse_targets(&subcommand.matches),
                profile: parse_profile(&subcommand.matches),
            },
            "run" => IosCommand::Run {
                profile: parse_profile(&subcommand.matches),
            },
            "list" => IosCommand::List,
            "compile-lib" => IosCommand::CompileLib {
                macos: subcommand.matches.is_present("macos"),
                arch: subcommand.matches.value_of("ARCH").unwrap().into(), // unwrap is fine, since clap makes sure we have this
                profile: parse_profile(&subcommand.matches),
            },
            _ => unreachable!(), // clap will reject anything else
        }
    }

    pub fn exec(self, config: &Config, noise_level: NoiseLevel) -> Result<(), Error> {
        fn detect_device<'a>(env: &'_ Env) -> Result<Device<'a>, Error> {
            let device_list = ios_deploy::device_list(env).map_err(Error::DeviceDetectionFailed)?;
            if device_list.len() > 0 {
                let index = if device_list.len() > 1 {
                    prompt::list(
                        "Detected iOS devices",
                        device_list.iter(),
                        "device",
                        None,
                        "Device",
                    )
                    .map_err(Error::DevicePromptFailed)?
                } else {
                    0
                };
                let device = device_list.into_iter().nth(index).unwrap();
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
            IosCommand::Check { targets } => call_for_targets_with_fallback(
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
            IosCommand::Build { targets, profile } => call_for_targets_with_fallback(
                targets.iter(),
                &detect_target_ok,
                &env,
                |target: &Target| {
                    target
                        .build(config, &env, profile)
                        .map_err(Error::BuildFailed)
                },
            )
            .map_err(Error::TargetInvalid)?,
            IosCommand::Run { profile } => detect_device(&env)?
                .run(config, &env, profile)
                .map_err(Error::RunFailed),
            IosCommand::List => ios_deploy::device_list(&env)
                .map_err(Error::ListFailed)
                .map(|device_list| {
                    if !device_list.is_empty() {
                        for (index, device) in device_list.iter().enumerate() {
                            println!("  [{}] {}", index, device);
                        }
                    } else {
                        println!("  No devices detected.");
                    }
                }),
            IosCommand::CompileLib {
                macos,
                arch,
                profile,
            } => match macos {
                true => Target::macos().compile_lib(config, noise_level, profile),
                false => Target::for_arch(&arch)
                    .ok_or_else(|| Error::ArchInvalid {
                        arch: arch.to_owned(),
                    })?
                    .compile_lib(config, noise_level, profile),
            }
            .map_err(Error::CompileLibFailed),
        }
    }
}
