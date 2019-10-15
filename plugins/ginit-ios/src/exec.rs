use crate::{
    config::Config,
    device::{Device, RunError},
    ios_deploy, project,
    target::{BuildError, CheckError, CompileLibError, Target},
};
use ginit_core::{
    cli_app,
    config::ConfigTrait as _,
    define_device_prompt,
    device::PromptError,
    env::{Env, Error as EnvError},
    exports::clap::{App, AppSettings, Arg, ArgMatches, SubCommand},
    opts,
    target::{call_for_targets_with_fallback, TargetInvalid},
    util::{cli, prompt},
};
use std::fmt::{self, Display};

pub fn app<'a, 'b>(targets: &'a [&'a str]) -> App<'a, 'b> {
    cli_app!(crate::NAME)
        .subcommand(
            SubCommand::with_name("check")
                .about("Checks if code compiles for target(s)")
                .display_order(1)
                .arg(cli::take_a_target_list::<Target>(targets)),
        )
        .subcommand(
            SubCommand::with_name("build")
                .about("Builds static libraries for target(s)")
                .display_order(2)
                .arg(cli::take_a_target_list::<Target>(targets))
                .arg_from_usage("--release 'Build with release optimizations'"),
        )
        .subcommand(
            SubCommand::with_name("run")
                .about("Deploys IPA to connected device")
                .display_order(3)
                .arg_from_usage("--release 'Build with release optimizations'"),
        )
        .subcommand(
            SubCommand::with_name("list")
                .about("Lists connected devices")
                .display_order(4),
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
    DevicePromptFailed(PromptError<ios_deploy::DeviceListError>),
    TargetInvalid(TargetInvalid),
    InitFailed(project::Error),
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
            Self::EnvInitFailed(err) => write!(f, "{}", err),
            Self::DevicePromptFailed(err) => write!(f, "{}", err),
            Self::TargetInvalid(err) => write!(f, "Specified target was invalid: {}", err),
            Self::InitFailed(err) => write!(f, "{}", err),
            Self::CheckFailed(err) => write!(f, "{}", err),
            Self::BuildFailed(err) => write!(f, "{}", err),
            Self::RunFailed(err) => write!(f, "{}", err),
            Self::ListFailed(err) => write!(f, "{}", err),
            Self::ArchInvalid { arch } => write!(f, "Specified arch was invalid: {}", arch),
            Self::CompileLibFailed(err) => write!(f, "{}", err),
        }
    }
}

#[derive(Debug)]
pub enum Command {
    Init {
        clobbering: opts::Clobbering,
    },
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
    List,
    CompileLib {
        macos: bool,
        arch: String,
        profile: opts::Profile,
    },
}

impl cli::CommandTrait for Command {
    fn parse(matches: &ArgMatches<'_>) -> Self {
        let subcommand = matches.subcommand.as_ref().unwrap(); // clap makes sure we got a subcommand
        match subcommand.name.as_str() {
            "init" => Self::Init {
                clobbering: cli::parse_clobbering(&subcommand.matches),
            },
            "check" => Self::Check {
                targets: cli::parse_targets(&subcommand.matches),
            },
            "build" => Self::Build {
                targets: cli::parse_targets(&subcommand.matches),
                profile: cli::parse_profile(&subcommand.matches),
            },
            "run" => Self::Run {
                profile: cli::parse_profile(&subcommand.matches),
            },
            "list" => Self::List,
            "compile-lib" => Self::CompileLib {
                macos: subcommand.matches.is_present("macos"),
                arch: subcommand.matches.value_of("ARCH").unwrap().into(), // unwrap is fine, since clap makes sure we have this
                profile: cli::parse_profile(&subcommand.matches),
            },
            _ => unreachable!(), // clap will reject anything else
        }
    }
}

pub fn exec(
    cli::Input {
        noise_level,
        interactivity,
        command,
    }: cli::Input<Command>,
    config: &Config,
) -> Result<(), Error> {
    define_device_prompt!(ios_deploy::device_list, ios_deploy::DeviceListError, iOS);
    fn detect_target_ok<'a>(env: &Env) -> Option<&'a Target<'a>> {
        device_prompt(env).map(|device| device.target()).ok()
    }

    let env = Env::new().map_err(Error::EnvInitFailed)?;
    match command {
        Command::Init { clobbering } => {
            project::generate(config, &config.init_templating(), clobbering)
                .map_err(Error::InitFailed)
        }
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
                    .build(config, &env, profile)
                    .map_err(Error::BuildFailed)
            },
        )
        .map_err(Error::TargetInvalid)?,
        Command::Run { profile } => device_prompt(&env)
            .map_err(Error::DevicePromptFailed)?
            .run(config, &env, profile)
            .map_err(Error::RunFailed),
        Command::List => ios_deploy::device_list(&env)
            .map_err(Error::ListFailed)
            .map(|device_list| {
                prompt::list_display_only(device_list.iter(), device_list.len());
            }),
        Command::CompileLib {
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
