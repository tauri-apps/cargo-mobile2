use crate::{
    adb,
    config::Config,
    device::{Device, RunError, StacktraceError},
    env::{Env, Error as EnvError},
    target::{BuildError, CompileLibError, Target},
};
use ginit_core::{
    define_device_prompt,
    device::PromptError,
    exports::clap::{App, AppSettings, ArgMatches, SubCommand},
    opts,
    target::{call_for_targets_with_fallback, TargetInvalid},
    util::{
        cli::{parse_profile, parse_targets, take_a_target_list},
        prompt,
    },
};
use std::fmt::{self, Display};

#[derive(Debug)]
pub enum Error {
    EnvInitFailed(EnvError),
    DevicePromptFailed(PromptError<adb::DeviceListError>),
    TargetInvalid(TargetInvalid),
    CheckFailed(CompileLibError),
    BuildFailed(BuildError),
    RunFailed(RunError),
    StacktraceFailed(StacktraceError),
    ListFailed(adb::DeviceListError),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EnvInitFailed(err) => write!(f, "{}", err),
            Self::DevicePromptFailed(err) => write!(f, "{}", err),
            Self::TargetInvalid(err) => write!(f, "Specified target was invalid: {}", err),
            Self::CheckFailed(err) => write!(f, "{}", err),
            Self::BuildFailed(err) => write!(f, "{}", err),
            Self::RunFailed(err) => write!(f, "{}", err),
            Self::StacktraceFailed(err) => write!(f, "{}", err),
            Self::ListFailed(err) => write!(f, "{}", err),
        }
    }
}

#[derive(Debug)]
pub enum Command {
    Check {
        targets: Vec<String>,
    },
    Build {
        targets: Vec<String>,
        profile: opts::Profile,
    },
    Run {
        profile: opts::Profile,
    },
    Stacktrace,
    List,
}

impl Command {
    pub fn app<'a, 'b>(targets: &'a [&'a str]) -> App<'a, 'b> {
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

    pub fn parse(matches: ArgMatches<'_>) -> Self {
        let subcommand = matches.subcommand.as_ref().unwrap(); // clap makes sure we got a subcommand
        match subcommand.name.as_str() {
            "check" => Self::Check {
                targets: parse_targets(&subcommand.matches),
            },
            "build" => Self::Build {
                targets: parse_targets(&subcommand.matches),
                profile: parse_profile(&subcommand.matches),
            },
            "run" => Self::Run {
                profile: parse_profile(&subcommand.matches),
            },
            "st" => Self::Stacktrace,
            "list" => Self::List,
            _ => unreachable!(), // clap will reject anything else
        }
    }

    pub fn exec(self, config: &Config, noise_level: opts::NoiseLevel) -> Result<(), Error> {
        define_device_prompt!(adb::device_list, adb::DeviceListError, Android);
        fn detect_target_ok<'a>(env: &Env) -> Option<&'a Target<'a>> {
            device_prompt(env).map(|device| device.target()).ok()
        }

        let env = Env::new().map_err(Error::EnvInitFailed)?;
        match self {
            Command::Check { targets } => call_for_targets_with_fallback(
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
            Command::Build { targets, profile } => call_for_targets_with_fallback(
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
            Command::Run { profile } => device_prompt(&env)
                .map_err(Error::DevicePromptFailed)?
                .run(config, &env, noise_level, profile)
                .map_err(Error::RunFailed),
            Command::Stacktrace => device_prompt(&env)
                .map_err(Error::DevicePromptFailed)?
                .stacktrace(config, &env)
                .map_err(Error::StacktraceFailed),
            Command::List => adb::device_list(&env)
                .map_err(Error::ListFailed)
                .map(|device_list| {
                    prompt::list_display_only(device_list.iter(), device_list.len());
                }),
        }
    }
}
