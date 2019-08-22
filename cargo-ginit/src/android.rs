use crate::util::{parse_profile, parse_targets, take_a_list};
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use ginit::{
    android::{ndk, target::Target},
    config::Config,
    opts::NoiseLevel,
    target::{call_for_targets, FallbackBehavior, Profile},
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

    pub fn exec(self, config: &Config, noise_level: NoiseLevel) {
        fn try_detect_target<'a>() -> Option<&'a Target<'a>> {
            let target = Target::for_connected()
                .ok()
                .and_then(std::convert::identity);
            if let Some(target) = target {
                println!("Detected target for connected device: {}", target.triple);
            }
            target
        }

        fn detect_target<'a>() -> &'a Target<'a> {
            try_detect_target().expect("Failed to detect target for connected device")
        }

        let ndk_env = ndk::Env::new().expect("Failed to init NDK env");
        match self {
            AndroidCommand::Check { targets } => call_for_targets(
                Some(targets.iter()),
                FallbackBehavior::get_target(&try_detect_target, true),
                |target: &Target| target.check(config, &ndk_env, noise_level),
            ),
            AndroidCommand::Build { targets, profile } => call_for_targets(
                Some(targets.iter()),
                FallbackBehavior::get_target(&try_detect_target, true),
                |target: &Target| target.build(config, &ndk_env, noise_level, profile),
            ),
            AndroidCommand::Run { profile } => {
                detect_target().run(config, &ndk_env, noise_level, profile)
            }
            AndroidCommand::Stacktrace => detect_target().stacktrace(config, &ndk_env),
        }
    }
}
