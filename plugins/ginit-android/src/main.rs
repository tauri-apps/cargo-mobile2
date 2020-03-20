#![forbid(unsafe_code)]

mod adb;
mod config;
mod device;
mod env;
mod ndk;
mod project;
mod target;

use self::{
    config::{Config, Raw},
    device::{Device, RunError, StacktraceError},
    env::{Env, Error as EnvError},
    target::{BuildError, CompileLibError, Target},
};
use ginit_core::{
    config::{gen as config_gen, ConfigTrait as _},
    define_device_prompt,
    device::PromptError,
    exports::bossy,
    os,
    target::{call_for_targets_with_fallback, TargetInvalid, TargetTrait as _},
    util::{self, cli, prompt},
};
use std::fmt::{self, Display};
use structopt::{clap::AppSettings, StructOpt};

static NAME: &'static str = "android";

#[cli::main(NAME)]
#[derive(Debug, StructOpt)]
#[structopt(bin_name = cli::bin_name(NAME), settings = cli::SETTINGS)]
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
    #[structopt(name = "open", about = "Open project in Android Studio")]
    Open,
    #[structopt(name = "check", about = "Checks if code compiles for target(s)")]
    Check {
        #[structopt(name = "targets", default_value = Target::DEFAULT_KEY, possible_values = Target::name_list())]
        targets: Vec<String>,
    },
    #[structopt(name = "build", about = "Builds dynamic libraries for target(s)")]
    Build {
        #[structopt(name = "targets", default_value = Target::DEFAULT_KEY, possible_values = Target::name_list())]
        targets: Vec<String>,
        #[structopt(flatten)]
        profile: cli::Profile,
    },
    #[structopt(name = "run", about = "Deploys APK to connected device")]
    Run {
        #[structopt(flatten)]
        profile: cli::Profile,
    },
    #[structopt(name = "st", about = "Displays a detailed stacktrace for a device")]
    Stacktrace,
    #[structopt(name = "list", about = "Lists connected devices")]
    List,
}

#[derive(Debug)]
pub enum Error {
    EnvInitFailed(EnvError),
    DevicePromptFailed(PromptError<adb::device_list::Error>),
    TargetInvalid(TargetInvalid),
    ConfigGenFailed(config_gen::Error<Raw>),
    ConfigRequired,
    InitFailed(project::Error),
    OpenFailed(bossy::Error),
    CheckFailed(CompileLibError),
    BuildFailed(BuildError),
    RunFailed(RunError),
    StacktraceFailed(StacktraceError),
    ListFailed(adb::device_list::Error),
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
            Self::OpenFailed(err) => write!(f, "Failed to open project in Android Studio: {}", err),
            Self::CheckFailed(err) => write!(f, "{}", err),
            Self::BuildFailed(err) => write!(f, "{}", err),
            Self::RunFailed(err) => write!(f, "{}", err),
            Self::StacktraceFailed(err) => write!(f, "{}", err),
            Self::ListFailed(err) => write!(f, "{}", err),
        }
    }
}

impl cli::Exec for Input {
    type Config = Config;
    type Error = Error;

    fn global_flags(&self) -> cli::GlobalFlags {
        self.flags
    }

    fn exec(
        self,
        config: Option<Self::Config>,
        wrapper: &util::TextWrapper,
    ) -> Result<(), Self::Error> {
        define_device_prompt!(adb::device_list, adb::device_list::Error, Android);
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
                project::generate(config, &env, &config.init_templating(), clobbering)
                    .map_err(Error::InitFailed)
            }),
            Command::Open => with_config(config, |config| {
                os::open_file_with("Android Studio", config.project_path())
                    .map_err(Error::OpenFailed)
            }),
            Command::Check { targets } => with_config(config, |config| {
                call_for_targets_with_fallback(
                    targets.iter(),
                    &detect_target_ok,
                    &env,
                    |target: &Target| {
                        target
                            .check(config, &env, noise_level, interactivity)
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
                            .build(config, &env, noise_level, interactivity, profile)
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
                    .run(config, &env, noise_level, profile)
                    .map_err(Error::RunFailed)
            }),
            Command::Stacktrace => with_config(config, |config| {
                device_prompt(&env)
                    .map_err(Error::DevicePromptFailed)?
                    .stacktrace(config, &env)
                    .map_err(Error::StacktraceFailed)
            }),
            Command::List => adb::device_list(&env)
                .map_err(Error::ListFailed)
                .map(|device_list| {
                    prompt::list_display_only(device_list.iter(), device_list.len());
                }),
        }
    }
}
