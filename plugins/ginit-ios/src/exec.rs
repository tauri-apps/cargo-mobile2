use crate::{
    config::{raw::Raw, Config},
    device::{Device, RunError},
    ios_deploy, project,
    target::{BuildError, CheckError, CompileLibError, Target},
};
use ginit_core::{
    cli_app,
    config::{self, ConfigTrait as _},
    define_device_prompt,
    device::PromptError,
    env::{Env, Error as EnvError},
    exports::clap::{App, AppSettings, Arg, ArgMatches, SubCommand},
    opts,
    target::{call_for_targets_with_fallback, TargetInvalid},
    util::{self, cli, prompt},
};
use std::fmt::{self, Display};

pub fn app<'a, 'b>(targets: &'a [&'a str]) -> App<'a, 'b> {
    cli_app!()
        .subcommand(
            SubCommand::with_name("check")
                .about("Checks if code compiles for target(s)")
                .display_order(2)
                .arg(cli::take_a_target_list::<Target>(targets)),
        )
        .subcommand(
            SubCommand::with_name("build")
                .about("Builds static libraries for target(s)")
                .display_order(3)
                .arg(cli::take_a_target_list::<Target>(targets))
                .arg_from_usage("--release 'Build with release optimizations'"),
        )
        .subcommand(
            SubCommand::with_name("run")
                .about("Deploys IPA to connected device")
                .display_order(4)
                .arg_from_usage("--release 'Build with release optimizations'"),
        )
        .subcommand(
            SubCommand::with_name("list")
                .about("Lists connected devices")
                .display_order(5),
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
    ConfigGenFailed(config::gen::Error<Raw>),
    ConfigRequired,
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
            Self::ConfigGenFailed(err) => write!(f, "Failed to generate config: {}", err),
            Self::ConfigRequired => write!(
                f,
                "Plugin is unconfigured, but configuration is required for this command."
            ),
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
    ConfigGen,
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
            "config-gen" => Self::ConfigGen,
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
    config: Option<&Config>,
    wrapper: &util::TextWrapper,
) -> Result<(), Error> {
    define_device_prompt!(ios_deploy::device_list, ios_deploy::DeviceListError, iOS);
    fn detect_target_ok<'a>(env: &Env) -> Option<&'a Target<'a>> {
        device_prompt(env).map(|device| device.target()).ok()
    }

    fn with_config(
        config: Option<&Config>,
        f: impl FnOnce(&Config) -> Result<(), Error>,
    ) -> Result<(), Error> {
        f(config.ok_or_else(|| Error::ConfigRequired)?)
    }

    let env = Env::new().map_err(Error::EnvInitFailed)?;
    match command {
        Command::ConfigGen => config::gen::detect_or_prompt(interactivity, wrapper, crate::NAME)
            .map_err(Error::ConfigGenFailed),
        Command::Init { clobbering } => with_config(config, |config| {
            project::generate(config, &config.init_templating(), clobbering)
                .map_err(Error::InitFailed)
        }),
        Command::Check { targets } => with_config(config, |config| {
            call_for_targets_with_fallback(
                targets.iter(),
                &detect_target_ok,
                &env,
                |target: &Target| {
                    target
                        .check(config, &env, noise_level)
                        .map_err(Error::CheckFailed)
                },
            )
            .map_err(Error::TargetInvalid)?
        }),
        Command::Build { targets, profile } => with_config(config, |config| {
            call_for_targets_with_fallback(
                targets.iter(),
                &detect_target_ok,
                &env,
                |target: &Target| {
                    target
                        .build(config, &env, profile)
                        .map_err(Error::BuildFailed)
                },
            )
            .map_err(Error::TargetInvalid)?
        }),
        Command::Run { profile } => with_config(config, |config| {
            device_prompt(&env)
                .map_err(Error::DevicePromptFailed)?
                .run(config, &env, profile)
                .map_err(Error::RunFailed)
        }),
        Command::List => ios_deploy::device_list(&env)
            .map_err(Error::ListFailed)
            .map(|device_list| {
                prompt::list_display_only(device_list.iter(), device_list.len());
            }),
        Command::CompileLib {
            macos,
            arch,
            profile,
        } => with_config(config, |config| {
            match macos {
                true => Target::macos().compile_lib(config, noise_level, profile),
                false => Target::for_arch(&arch)
                    .ok_or_else(|| Error::ArchInvalid {
                        arch: arch.to_owned(),
                    })?
                    .compile_lib(config, noise_level, profile),
            }
            .map_err(Error::CompileLibFailed)
        }),
    }
}
