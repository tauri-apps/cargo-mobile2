use crate::util::{parse_release, parse_targets, take_a_list};
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use ginit::{
    android::{ndk, target::Target},
    config::Config,
    target::{call_for_targets, FallbackBehavior},
};

pub fn subcommand<'a, 'b>(targets: &'a [&'a str]) -> App<'a, 'b> {
    SubCommand::with_name("android")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .about("Tools for Android")
        .subcommand(
            SubCommand::with_name("check")
                .about("Checks if code compiles for target(s)")
                .display_order(0)
                .arg(take_a_list(Arg::with_name("TARGETS"), targets)),
        )
        .subcommand(
            SubCommand::with_name("build")
                .about("Builds dynamic libraries for target(s)")
                .display_order(1)
                .arg(take_a_list(Arg::with_name("TARGETS"), targets))
                .arg_from_usage("--release 'Build with release optimizations'"),
        )
        .subcommand(
            SubCommand::with_name("run")
                .about("Deploys APK for target(s)")
                .display_order(2)
                .arg(take_a_list(Arg::with_name("TARGETS"), targets))
                .arg_from_usage("--release 'Build with release optimizations'"),
        )
        .subcommand(
            SubCommand::with_name("st")
                .display_order(3)
                .about("Displays a detailed stacktrace for a target")
                .arg(Arg::with_name("TARGET").possible_values(targets)),
        )
}

#[derive(Debug)]
pub enum AndroidCommand {
    Check { targets: Vec<String> },
    Build { targets: Vec<String>, release: bool },
    Run { targets: Vec<String>, release: bool },
    Stacktrace { target: Option<String> },
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
                release: parse_release(&subcommand.matches),
            },
            "run" => AndroidCommand::Run {
                targets: parse_targets(&subcommand.matches),
                release: parse_release(&subcommand.matches),
            },
            "st" => AndroidCommand::Stacktrace {
                target: subcommand.matches.value_of("TARGET").map(Into::into),
            },
            _ => unreachable!(), // clap will reject anything else
        }
    }

    pub fn exec(self, config: &Config, verbose: bool) {
        fn detect_target<'a>() -> Option<&'a Target<'a>> {
            let target = Target::for_connected()
                .ok()
                .and_then(std::convert::identity);
            if let Some(target) = target {
                println!("Detected target for connected device: {}", target.triple);
            }
            target
        }

        let ndk_env = ndk::Env::new().expect("Failed to init NDK env");
        match self {
            AndroidCommand::Check { targets } => call_for_targets(
                Some(targets.iter()),
                FallbackBehavior::get_target(&detect_target, true),
                |target: &Target| target.check(config, &ndk_env, verbose),
            ),
            AndroidCommand::Build { targets, release } => call_for_targets(
                Some(targets.iter()),
                FallbackBehavior::get_target(&detect_target, true),
                |target: &Target| target.build(config, &ndk_env, verbose, release),
            ),
            AndroidCommand::Run { targets, release } => call_for_targets(
                Some(targets.iter()),
                FallbackBehavior::get_target(&detect_target, true),
                |target: &Target| target.run(config, &ndk_env, verbose, release),
            ),
            AndroidCommand::Stacktrace { target } => call_for_targets(
                target.as_ref().map(std::iter::once),
                FallbackBehavior::get_target(&detect_target, false),
                |target: &Target| target.stacktrace(config, &ndk_env),
            ),
        }
    }
}
