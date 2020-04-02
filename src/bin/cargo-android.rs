#![forbid(unsafe_code)]

use cargo_mobile::{
    android::{
        adb,
        config::Config,
        device::{Device, RunError, StacktraceError},
        env::{Env, Error as EnvError},
        target::{BuildError, CompileLibError, Target},
        NAME,
    },
    config::{Config as OmniConfig, LoadOrGenError},
    define_device_prompt,
    device::PromptError,
    init, opts, os,
    target::{call_for_targets_with_fallback, TargetInvalid, TargetTrait as _},
    util::{
        cli::{self, Exec, ExecError, GlobalFlags, TextWrapper},
        prompt,
    },
};
use std::fmt::{self, Display};
use structopt::StructOpt;

#[cli::main(NAME)]
#[derive(Debug, StructOpt)]
#[structopt(bin_name = cli::bin_name(NAME), settings = cli::SETTINGS)]
pub struct Input {
    #[structopt(flatten)]
    flags: GlobalFlags,
    #[structopt(subcommand)]
    command: Command,
}

#[derive(Debug, StructOpt)]
pub enum Command {
    #[structopt(
        name = "init",
        about = "Creates a new project in the current working directory"
    )]
    Init {
        #[structopt(flatten)]
        clobbering: cli::Clobbering,
        #[structopt(
            long,
            about = "Open in Android Studio",
            parse(from_flag = opts::OpenIn::from_flag),
        )]
        open: opts::OpenIn,
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
    ConfigFailed(LoadOrGenError),
    InitFailed(init::Error),
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
            Self::ConfigFailed(err) => write!(f, "Failed to load or generate config: {}", err),
            Self::InitFailed(err) => write!(f, "Failed to generate project: {}", err),
            Self::OpenFailed(err) => write!(f, "Failed to open project in Android Studio: {}", err),
            Self::CheckFailed(err) => write!(f, "{}", err),
            Self::BuildFailed(err) => write!(f, "{}", err),
            Self::RunFailed(err) => write!(f, "{}", err),
            Self::StacktraceFailed(err) => write!(f, "{}", err),
            Self::ListFailed(err) => write!(f, "{}", err),
        }
    }
}

impl ExecError for Error {}

impl Exec for Input {
    type Error = Error;

    fn global_flags(&self) -> GlobalFlags {
        self.flags
    }

    fn exec(self, wrapper: &TextWrapper) -> Result<(), Self::Error> {
        define_device_prompt!(adb::device_list, adb::device_list::Error, Android);
        fn detect_target_ok<'a>(env: &Env) -> Option<&'a Target<'a>> {
            device_prompt(env).map(|device| device.target()).ok()
        }

        fn with_config(
            interactivity: opts::Interactivity,
            wrapper: &TextWrapper,
            f: impl FnOnce(&Config) -> Result<(), Error>,
        ) -> Result<(), Error> {
            let config = OmniConfig::load_or_gen(".", interactivity, wrapper)
                .map_err(Error::ConfigFailed)?;
            f(config.android())
        }

        fn open_in_android_studio(config: &Config) -> Result<(), Error> {
            os::open_file_with("Android Studio", config.project_dir()).map_err(Error::OpenFailed)
        }

        let Self {
            flags:
                GlobalFlags {
                    noise_level,
                    interactivity,
                },
            command,
        } = self;
        let env = Env::new().map_err(Error::EnvInitFailed)?;
        match command {
            Command::Init {
                clobbering: cli::Clobbering { clobbering },
                open,
            } => {
                let config = init::exec(
                    interactivity,
                    clobbering,
                    opts::OpenIn::Nothing,
                    Some(vec!["android".into()]),
                    None,
                    wrapper,
                )
                .map_err(Error::InitFailed)?;
                if open.editor() {
                    open_in_android_studio(config.android())
                } else {
                    Ok(())
                }
            }
            Command::Open => with_config(interactivity, wrapper, open_in_android_studio),
            Command::Check { targets } => with_config(interactivity, wrapper, |config| {
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
            } => with_config(interactivity, wrapper, |config| {
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
            } => with_config(interactivity, wrapper, |config| {
                device_prompt(&env)
                    .map_err(Error::DevicePromptFailed)?
                    .run(config, &env, noise_level, profile)
                    .map_err(Error::RunFailed)
            }),
            Command::Stacktrace => with_config(interactivity, wrapper, |config| {
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
