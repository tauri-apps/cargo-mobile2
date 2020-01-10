#![forbid(unsafe_code)]

mod config;
mod deps;
mod device;
mod ios_deploy;
mod project;
mod system_profile;
mod target;
mod teams;

use self::{
    config::{raw::Raw, Config},
    device::{Device, RunError},
    target::{BuildError, CheckError, CompileLibError, Target},
};
use ginit_core::{
    config::{gen as config_gen, ConfigTrait as _},
    define_device_prompt,
    device::PromptError,
    env::{Env, Error as EnvError},
    target::{call_for_targets_with_fallback, TargetInvalid, TargetTrait as _},
    util::{self, cli, prompt},
};
use std::fmt::{self, Display};
use structopt::{clap::AppSettings, StructOpt};

static NAME: &'static str = "ios";

#[cli::main(NAME)]
#[derive(Debug, StructOpt)]
#[structopt(settings = cli::SETTINGS)]
pub struct Input {
    #[structopt(flatten)]
    flags: cli::GlobalFlags,
    #[structopt(subcommand)]
    command: Command,
}

#[derive(Debug, StructOpt)]
pub enum Command {
    #[structopt(name = "config-gen", about = "Generate configuration", setting = AppSettings::Hidden)]
    ConfigGen,
    #[structopt(
        name = "init",
        about = "Creates a new project in the current working directory"
    )]
    Init {
        #[structopt(flatten)]
        clobbering: cli::Clobbering,
    },
    #[structopt(name = "check", about = "Checks if code compiles for target(s)")]
    Check {
        #[structopt(name = "targets", default_value = Target::DEFAULT_KEY, possible_values = Target::name_list())]
        targets: Vec<String>,
    },
    #[structopt(name = "build", about = "Builds static libraries for target(s)")]
    Build {
        #[structopt(name = "targets", default_value = Target::DEFAULT_KEY, possible_values = Target::name_list())]
        targets: Vec<String>,
        #[structopt(flatten)]
        profile: cli::Profile,
    },
    #[structopt(name = "run", about = "Deploys IPA to connected device")]
    Run {
        #[structopt(flatten)]
        profile: cli::Profile,
    },
    #[structopt(name = "list", about = "Lists connected devices")]
    List,
    #[structopt(
        name = "compile-lib",
        about = "Compiles static lib (should only be called by Xcode!)",
        setting = AppSettings::Hidden
    )]
    CompileLib {
        #[structopt(long = "macos", about = "Awkwardly special-case for macOS")]
        macos: bool,
        #[structopt(name = "ARCH", index = 1, required = true)]
        arch: String,
        #[structopt(flatten)]
        profile: cli::Profile,
    },
}

#[derive(Debug)]
pub enum Error {
    EnvInitFailed(EnvError),
    DevicePromptFailed(PromptError<ios_deploy::DeviceListError>),
    TargetInvalid(TargetInvalid),
    ConfigGenFailed(config_gen::Error<Raw>),
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

impl cli::Exec for Input {
    type Config = Config;
    type Error = Error;

    fn exec(
        self,
        config: Option<Self::Config>,
        wrapper: &util::TextWrapper,
    ) -> Result<(), Self::Error> {
        define_device_prompt!(ios_deploy::device_list, ios_deploy::DeviceListError, iOS);
        fn detect_target_ok<'a>(env: &Env) -> Option<&'a Target<'a>> {
            device_prompt(env).map(|device| device.target()).ok()
        }

        fn with_config(
            config: Option<Config>,
            f: impl FnOnce(&Config) -> Result<(), Error>,
        ) -> Result<(), Error> {
            f(config.as_ref().ok_or_else(|| Error::ConfigRequired)?)
        }

        let Self {
            flags:
                cli::GlobalFlags {
                    noise_level,
                    interactivity,
                },
            command,
        } = self;
        let env = Env::new().map_err(Error::EnvInitFailed)?;
        match command {
            Command::ConfigGen => config_gen::detect_or_prompt(interactivity, wrapper, crate::NAME)
                .map_err(Error::ConfigGenFailed),
            Command::Init {
                clobbering: cli::Clobbering { clobbering },
            } => with_config(config, |config| {
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
            Command::Build {
                targets,
                profile: cli::Profile { profile },
            } => with_config(config, |config| {
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
            Command::Run {
                profile: cli::Profile { profile },
            } => with_config(config, |config| {
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
                profile: cli::Profile { profile },
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
}
