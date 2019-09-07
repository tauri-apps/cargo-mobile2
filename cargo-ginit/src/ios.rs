use crate::util::{parse_profile, parse_targets, take_a_target_list};
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use ginit::{
    config::Config,
    env::{Env, Error as EnvError},
    ios::target::{BuildError, CheckError, CompileLibError, RunError, Target},
    opts::NoiseLevel,
    target::{call_for_targets, Profile, TargetInvalid, TargetTrait as _},
};
use std::fmt;

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
    TargetInvalid(TargetInvalid),
    CheckFailed(CheckError),
    BuildFailed(BuildError),
    RunFailed(RunError),
    ArchInvalid { arch: String },
    CompileLibFailed(CompileLibError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::EnvInitFailed(err) => write!(f, "{}", err),
            Error::TargetInvalid(err) => write!(f, "Specified target was invalid: {}", err),
            Error::CheckFailed(err) => write!(f, "{}", err),
            Error::BuildFailed(err) => write!(f, "{}", err),
            Error::RunFailed(err) => write!(f, "{}", err),
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
            "compile-lib" => IosCommand::CompileLib {
                macos: subcommand.matches.is_present("macos"),
                arch: subcommand.matches.value_of("ARCH").unwrap().into(), // unwrap is fine, since clap makes sure we have this
                profile: parse_profile(&subcommand.matches),
            },
            _ => unreachable!(), // clap will reject anything else
        }
    }

    pub fn exec(self, config: &Config, noise_level: NoiseLevel) -> Result<(), Error> {
        let env = Env::new().map_err(Error::EnvInitFailed)?;
        match self {
            IosCommand::Check { targets } => call_for_targets(targets.iter(), |target: &Target| {
                target
                    .check(config, &env, noise_level)
                    .map_err(Error::CheckFailed)
            })
            .map_err(Error::TargetInvalid)?,
            IosCommand::Build { targets, profile } => {
                call_for_targets(targets.iter(), |target: &Target| {
                    target
                        .build(config, &env, profile)
                        .map_err(Error::BuildFailed)
                })
                .map_err(Error::TargetInvalid)?
            }
            IosCommand::Run { profile } => {
                // TODO: this isn't simulator-friendly, among other things
                Target::default_ref()
                    .run(config, &env, profile)
                    .map_err(Error::RunFailed)
            }
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
