use crate::util::{parse_profile, parse_targets, take_a_target_list};
use clap::{App, AppSettings, ArgMatches, SubCommand};
use ginit::{
    android::{
        env::{Env, Error as EnvError},
        target::{
            BuildError, CompileLibError, ConnectedTargetError, RunError, StacktraceError, Target,
        },
    },
    config::Config,
    opts::NoiseLevel,
    target::{call_for_targets_with_fallback, Profile, TargetInvalid},
};

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
}

#[derive(Debug)]
pub enum Error {
    EnvInitFailed(EnvError),
    TargetDetectionFailed(ConnectedTargetError),
    TargetInvalid(TargetInvalid),
    CheckFailed(CompileLibError),
    BuildFailed(BuildError),
    RunFailed(RunError),
    StacktraceFailed(StacktraceError),
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
            _ => unreachable!(), // clap will reject anything else
        }
    }

    pub fn exec(self, config: &Config, noise_level: NoiseLevel) -> Result<(), Error> {
        fn detect_target<'a>(env: &Env) -> Result<&'a Target<'a>, Error> {
            let target = Target::for_connected(env).map_err(Error::TargetDetectionFailed);
            if let Ok(target) = target {
                println!("Detected target for connected device: {}", target.triple);
            }
            target
        }

        fn detect_target_ok<'a>(env: &Env) -> Option<&'a Target<'a>> {
            detect_target(env).ok()
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
            AndroidCommand::Run { profile } => detect_target(&env)?
                .run(config, &env, noise_level, profile)
                .map_err(Error::RunFailed),
            AndroidCommand::Stacktrace => detect_target(&env)?
                .stacktrace(config, &env)
                .map_err(Error::StacktraceFailed),
        }
    }
}
