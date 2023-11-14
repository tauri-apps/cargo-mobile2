use crate::{
    android::{
        aab, adb, apk,
        config::{Config, Metadata},
        device::{Device, RunError, StacktraceError},
        env::{Env, Error as EnvError},
        target::{BuildError, CompileLibError, Target},
        DEFAULT_ACTIVITY, NAME,
    },
    config::{
        metadata::{self, Metadata as OmniMetadata},
        Config as OmniConfig, LoadOrGenError,
    },
    define_device_prompt,
    device::PromptError,
    os,
    target::{call_for_targets_with_fallback, TargetInvalid, TargetTrait as _},
    util::{
        cli::{
            self, Exec, GlobalFlags, Report, Reportable, TextWrapper, VERSION_LONG, VERSION_SHORT,
        },
        prompt,
    },
};
use std::{ffi::OsString, path::PathBuf};
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
        #[structopt(name = "targets", default_value = Target::DEFAULT_KEY, possible_values = &Target::name_list())]
        targets: Vec<String>,
    },
    #[structopt(name = "build", about = "Builds dynamic libraries for target(s)")]
    Build {
        #[structopt(name = "targets", default_value = Target::DEFAULT_KEY, possible_values = &Target::name_list())]
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
        #[structopt(flatten)]
        reinstall_deps: cli::ReinstallDeps,
        #[structopt(
            short = "a",
            long = "activity",
            help = "Specifies which activtiy to launch"
        )]
        activity: Option<String>,
    },
    #[structopt(name = "st", about = "Displays a detailed stacktrace for a device")]
    Stacktrace,
    #[structopt(name = "list", about = "Lists connected devices")]
    List,
    #[structopt(name = "apk", about = "Manage and build APKs")]
    Apk {
        #[structopt(subcommand)]
        cmd: ApkSubcommand,
    },
    #[structopt(name = "aab", about = "Manage and build AABs")]
    Aab {
        #[structopt(subcommand)]
        cmd: AabSubcommand,
    },
}

#[derive(StructOpt, Clone, Debug)]
pub enum ApkSubcommand {
    #[structopt(about = "build APKs (Android Package Kit)")]
    Build {
        #[structopt(name = "targets", possible_values = &Target::name_list())]
        /// Which targets to build (all by default).
        targets: Vec<String>,
        #[structopt(flatten)]
        profile: cli::Profile,
        #[structopt(long = "split-per-abi", help = "Whether to split the APKs per ABIs.")]
        split_per_abi: bool,
    },
}
#[derive(StructOpt, Clone, Debug)]
pub enum AabSubcommand {
    #[structopt(about = "build AABs (Android App Bundle)")]
    Build {
        #[structopt(name = "targets", possible_values = &Target::name_list())]
        /// Which targets to build (all by default).
        targets: Vec<String>,
        #[structopt(flatten)]
        profile: cli::Profile,
        #[structopt(long = "split-per-abi", help = "Whether to split the AABs per ABIs.")]
        split_per_abi: bool,
    },
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
    OpenFailed(os::OpenFileError),
    CheckFailed(CompileLibError),
    BuildFailed(BuildError),
    RunFailed(RunError),
    StacktraceFailed(StacktraceError),
    ListFailed(adb::device_list::Error),
    ApkError(apk::ApkError),
    AabError(aab::AabError),
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
            Self::ApkError(err) => err.report(),
            Self::AabError(err) => err.report(),
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
            non_interactive: bool,
            wrapper: &TextWrapper,
            f: impl FnOnce(&Config, &Metadata, &Env) -> Result<(), Error>,
        ) -> Result<(), Error> {
            let (config, _origin) = OmniConfig::load_or_gen(".", non_interactive, wrapper)
                .map_err(Error::ConfigFailed)?;
            let metadata =
                OmniMetadata::load(config.app().root_dir()).map_err(Error::MetadataFailed)?;
            let mut env = Env::new().map_err(Error::EnvInitFailed)?;

            if let Some(vars) = metadata.android().env_vars.as_ref() {
                env.base = env.base.explicit_env_vars(
                    vars.iter()
                        .map(|d| {
                            (
                                d.0.to_owned(),
                                OsString::from(
                                    d.1.replace(
                                        "<android-project-dir>",
                                        &dunce::simplified(&config.android().project_dir())
                                            .to_string_lossy(),
                                    ),
                                ),
                            )
                        })
                        .collect::<std::collections::HashMap<_, _>>(),
                );
            }

            if metadata.android().supported() {
                f(config.android(), metadata.android(), &env)
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

        fn open_in_android_studio(config: &Config, env: &Env) -> Result<(), Error> {
            os::open_file_with("Android Studio", config.project_dir(), &env.base)
                .map_err(Error::OpenFailed)
        }

        fn get_targets_or_all<'a>(targets: Vec<String>) -> Result<Vec<&'a Target<'a>>, Error> {
            if targets.is_empty() {
                Ok(Target::all().iter().map(|t| t.1).collect())
            } else {
                let mut outs = Vec::new();
                for t in targets {
                    let target = Target::for_name(&t)
                        .ok_or_else(|| TargetInvalid {
                            name: t,
                            possible: Target::all().keys().map(|key| key.to_string()).collect(),
                        })
                        .map_err(Error::TargetInvalid)?;
                    outs.push(target);
                }
                Ok(outs)
            }
        }

        let Self {
            flags:
                GlobalFlags {
                    noise_level,
                    non_interactive,
                },
            command,
        } = self;
        match command {
            Command::Open => with_config(non_interactive, wrapper, |config, _, env| {
                ensure_init(config)?;
                open_in_android_studio(config, env)
            }),
            Command::Check { targets } => {
                with_config(non_interactive, wrapper, |config, metadata, env| {
                    let force_color = true;
                    call_for_targets_with_fallback(
                        targets.iter(),
                        &detect_target_ok,
                        env,
                        |target: &Target| {
                            target
                                .check(config, metadata, env, noise_level, force_color)
                                .map_err(Error::CheckFailed)
                        },
                    )
                    .map_err(Error::TargetInvalid)?
                })
            }
            Command::Build {
                targets,
                profile: cli::Profile { profile },
            } => with_config(non_interactive, wrapper, |config, metadata, env| {
                ensure_init(config)?;
                let force_color = true;
                call_for_targets_with_fallback(
                    targets.iter(),
                    &detect_target_ok,
                    env,
                    |target: &Target| {
                        target
                            .build(config, metadata, env, noise_level, force_color, profile)
                            .map_err(Error::BuildFailed)
                    },
                )
                .map_err(Error::TargetInvalid)?
            }),
            Command::Run {
                profile: cli::Profile { profile },
                filter: cli::Filter { filter },
                reinstall_deps: cli::ReinstallDeps { reinstall_deps },
                activity,
            } => with_config(non_interactive, wrapper, |config, metadata, env| {
                let build_app_bundle = metadata.asset_packs().is_some();
                ensure_init(config)?;
                device_prompt(env)
                    .map_err(Error::DevicePromptFailed)?
                    .run(
                        config,
                        env,
                        noise_level,
                        profile,
                        filter,
                        build_app_bundle,
                        reinstall_deps,
                        activity.unwrap_or_else(|| {
                            metadata
                                .app_activity_name()
                                .unwrap_or(DEFAULT_ACTIVITY)
                                .to_string()
                        }),
                    )
                    .and_then(|h| h.wait().map(|_| ()).map_err(Into::into))
                    .map_err(Error::RunFailed)
            }),
            Command::Stacktrace => with_config(non_interactive, wrapper, |config, _, env| {
                ensure_init(config)?;
                device_prompt(env)
                    .map_err(Error::DevicePromptFailed)?
                    .stacktrace(config, env)
                    .map_err(Error::StacktraceFailed)
            }),
            Command::List => with_config(non_interactive, wrapper, |_, _, env| {
                adb::device_list(env)
                    .map_err(Error::ListFailed)
                    .map(|device_list| {
                        prompt::list_display_only(device_list.iter(), device_list.len());
                    })
            }),
            Command::Apk { cmd } => match cmd {
                ApkSubcommand::Build {
                    targets,
                    profile: cli::Profile { profile },
                    split_per_abi,
                } => with_config(non_interactive, wrapper, |config, _, env| {
                    ensure_init(config)?;

                    apk::cli::build(
                        config,
                        env,
                        noise_level,
                        profile,
                        get_targets_or_all(targets)?,
                        split_per_abi,
                    )
                    .map_err(Error::ApkError)
                }),
            },
            Command::Aab { cmd } => match cmd {
                AabSubcommand::Build {
                    targets,
                    profile: cli::Profile { profile },
                    split_per_abi,
                } => with_config(non_interactive, wrapper, |config, _, env| {
                    ensure_init(config)?;
                    aab::cli::build(
                        config,
                        env,
                        noise_level,
                        profile,
                        get_targets_or_all(targets)?,
                        split_per_abi,
                    )
                    .map_err(Error::AabError)
                }),
            },
        }
    }
}
