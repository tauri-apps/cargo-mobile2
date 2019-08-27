use crate::util::{parse_profile, parse_targets, take_a_list};
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use ginit::{
    config::Config,
    ios::target::Target,
    opts::NoiseLevel,
    target::{call_for_targets, Profile, TargetTrait as _},
};

pub fn subcommand<'a, 'b>(targets: &'a [&'a str]) -> App<'a, 'b> {
    SubCommand::with_name("ios")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .about("Tools for iOS")
        .subcommand(
            SubCommand::with_name("check")
                .about("Checks if code compiles for target(s)")
                .display_order(0)
                .arg(take_a_list(Arg::with_name("TARGETS"), targets)),
        )
        .subcommand(
            SubCommand::with_name("build")
                .about("Builds static library")
                .display_order(1)
                .arg(take_a_list(Arg::with_name("TARGETS"), targets))
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
pub enum IOSCommand {
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

impl IOSCommand {
    pub fn parse(matches: ArgMatches<'_>) -> Self {
        let subcommand = matches.subcommand.as_ref().unwrap(); // clap makes sure we got a subcommand
        match subcommand.name.as_str() {
            "check" => IOSCommand::Check {
                targets: parse_targets(&subcommand.matches),
            },
            "build" => IOSCommand::Build {
                targets: parse_targets(&subcommand.matches),
                profile: parse_profile(&subcommand.matches),
            },
            "run" => IOSCommand::Run {
                profile: parse_profile(&subcommand.matches),
            },
            "compile-lib" => IOSCommand::CompileLib {
                macos: subcommand.matches.is_present("macos"),
                arch: subcommand.matches.value_of("ARCH").unwrap().into(), // unwrap is fine, since clap makes sure we have this
                profile: parse_profile(&subcommand.matches),
            },
            _ => unreachable!(), // clap will reject anything else
        }
    }

    pub fn exec(self, config: &Config, noise_level: NoiseLevel) {
        match self {
            IOSCommand::Check { targets } => {
                call_for_targets(Some(targets.iter()), None, |target: &Target| {
                    target.check(config, noise_level)
                })
            }
            IOSCommand::Build { targets, profile } => {
                call_for_targets(Some(targets.iter()), None, |target: &Target| {
                    target.build(config, profile)
                })
            }
            IOSCommand::Run { profile } => {
                // TODO: this isn't simulator-friendly, among other things
                Target::default_ref().run(config, profile)
            }
            IOSCommand::CompileLib {
                macos,
                arch,
                profile,
            } => match macos {
                true => Target::macos().compile_lib(config, noise_level, profile),
                false => Target::for_arch(&arch)
                    .expect("Invalid architecture")
                    .compile_lib(config, noise_level, profile),
            },
        }
    }
}
