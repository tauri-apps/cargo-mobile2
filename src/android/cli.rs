use crate::{
    android::{
        adb,
        config::{Config, Metadata},
        device::{Device, RunError, StacktraceError},
        env::{Env, Error as EnvError},
        target::{BuildError, CompileLibError, Target},
        NAME,
    },
    config::{
        metadata::{self, Metadata as OmniMetadata},
        Config as OmniConfig, LoadOrGenError,
    },
    define_device_prompt,
    device::PromptError,
    opts, os,
    target::{call_for_targets_with_fallback, TargetInvalid, TargetTrait as _},
    util::{
        cli::{
            self, Exec, GlobalFlags, Report, Reportable, TextWrapper, VERSION_LONG, VERSION_SHORT,
        },
        prompt,
    },
};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    bin_name = cli::bin_name(NAME),
    version = VERSION_SHORT,
    long_version = VERSION_LONG.as_str(),
    global_settings = cli::GLOBAL_SETTINGS,
    settings = cli::SETTINGS,
)]
pub struct Input {
    #[structopt(flatten)]
    flags: GlobalFlags,
    #[structopt(subcommand)]
    command: Command,
}

impl Input {
    pub fn new(flags: GlobalFlags, command: Command) -> Self {
        Self { flags, command }
    }
}

#[derive(Clone, Debug, StructOpt)]
pub enum Command {
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
        #[structopt(flatten)]
        filter: cli::Filter,
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
    MetadataFailed(metadata::Error),
    Unsupported,
    ProjectDirAbsent { project_dir: PathBuf },
    OpenFailed(bossy::Error),
    CheckFailed(CompileLibError),
    BuildFailed(BuildError),
    RunFailed(RunError),
    StacktraceFailed(StacktraceError),
    ListFailed(adb::device_list::Error),
}

impl Reportable for Error {
    fn report(&self) -> Report {
        match self {
            Self::EnvInitFailed(err) => err.report(),
            Self::DevicePromptFailed(err) => err.report(),
            Self::TargetInvalid(err) => Report::error("Specified target was invalid", err),
            Self::ConfigFailed(err) => err.report(),
            Self::MetadataFailed(err) => err.report(),
            Self::Unsupported => Report::error("Android is marked as unsupported in your Cargo.toml metadata", "If your project should support Android, modify your Cargo.toml, then run `cargo mobile init` and try again."),
            Self::ProjectDirAbsent { project_dir } => Report::action_request(
                "Please run `cargo mobile init` and try again!",
                format!(
                    "Android Studio project directory {:?} doesn't exist.",
                    project_dir
                ),
            ),
            Self::OpenFailed(err) => Report::error("Failed to open project in Android Studio", err),
            Self::CheckFailed(err) => err.report(),
            Self::BuildFailed(err) => err.report(),
            Self::RunFailed(err) => err.report(),
            Self::StacktraceFailed(err) => err.report(),
            Self::ListFailed(err) => err.report(),
        }
    }
}

impl Exec for Input {
    type Report = Error;

    fn global_flags(&self) -> GlobalFlags {
        self.flags
    }

    fn exec(self, wrapper: &TextWrapper) -> Result<(), Self::Report> {
        define_device_prompt!(adb::device_list, adb::device_list::Error, Android);
        fn detect_target_ok<'a>(env: &Env) -> Option<&'a Target<'a>> {
            device_prompt(env).map(|device| device.target()).ok()
        }

        fn with_config(
            non_interactive: opts::NonInteractive,
            wrapper: &TextWrapper,
            f: impl FnOnce(&Config, &Metadata) -> Result<(), Error>,
        ) -> Result<(), Error> {
            let (config, _origin) = OmniConfig::load_or_gen(".", non_interactive, wrapper)
                .map_err(Error::ConfigFailed)?;
            let metadata =
                OmniMetadata::load(&config.app().root_dir()).map_err(Error::MetadataFailed)?;
            if metadata.android().supported() {
                f(config.android(), metadata.android())
            } else {
                Err(Error::Unsupported)
            }
        }

        fn ensure_init(config: &Config) -> Result<(), Error> {
            if !config.project_dir_exists() {
                Err(Error::ProjectDirAbsent {
                    project_dir: config.project_dir(),
                })
            } else {
                Ok(())
            }
        }

        fn open_in_android_studio(config: &Config) -> Result<(), Error> {
            os::open_file_with("Android Studio", config.project_dir()).map_err(Error::OpenFailed)
        }

        let Self {
            flags:
                GlobalFlags {
                    noise_level,
                    non_interactive,
                },
            command,
        } = self;
        let env = Env::new().map_err(Error::EnvInitFailed)?;
        match command {
            Command::Open => with_config(non_interactive, wrapper, |config, _| {
                ensure_init(config)?;
                open_in_android_studio(config)
            }),
            Command::Check { targets } => {
                with_config(non_interactive, wrapper, |config, metadata| {
                    let force_color = opts::ForceColor::Yes;
                    call_for_targets_with_fallback(
                        targets.iter(),
                        &detect_target_ok,
                        &env,
                        |target: &Target| {
                            target
                                .check(config, metadata, &env, noise_level, force_color)
                                .map_err(Error::CheckFailed)
                        },
                    )
                    .map_err(Error::TargetInvalid)?
                })
            }
            Command::Build {
                targets,
                profile: cli::Profile { profile },
            } => with_config(non_interactive, wrapper, |config, metadata| {
                ensure_init(config)?;
                let force_color = opts::ForceColor::Yes;
                call_for_targets_with_fallback(
                    targets.iter(),
                    &detect_target_ok,
                    &env,
                    |target: &Target| {
                        target
                            .build(config, metadata, &env, noise_level, force_color, profile)
                            .map_err(Error::BuildFailed)
                    },
                )
                .map_err(Error::TargetInvalid)?
            }),
            Command::Run {
                profile: cli::Profile { profile },
                filter: cli::Filter { filter },
            } => with_config(non_interactive, wrapper, |config, _| {
                ensure_init(config)?;
                device_prompt(&env)
                    .map_err(Error::DevicePromptFailed)?
                    .run(config, &env, noise_level, profile, filter)
                    .map_err(Error::RunFailed)
            }),
            Command::Stacktrace => with_config(non_interactive, wrapper, |config, _| {
                ensure_init(config)?;
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
