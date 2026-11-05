use crate::{
    android::{
        aab, adb, apk,
        config::{Config, Metadata},
        device::{ConnectionStatus, Device, RunError, StacktraceError},
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
    opts, os,
    target::{call_for_targets_with_fallback, TargetInvalid, TargetTrait as _},
    util::{
        cli::{
            self, Exec, GlobalFlags, Report, Reportable, TextWrapper, VERSION_LONG, VERSION_SHORT,
        },
        prompt,
    },
};
use clap::{Parser, Subcommand};
use std::{ffi::OsString, path::PathBuf};

#[derive(Debug, Parser)]
#[clap(
    bin_name = cli::bin_name(NAME),
    version = VERSION_SHORT,
    long_version = VERSION_LONG.as_str(),
    subcommand_required = true,
    arg_required_else_help = true,
)]
pub struct Input {
    #[clap(flatten)]
    flags: GlobalFlags,
    #[clap(subcommand)]
    command: Command,
}

impl Input {
    pub fn new(flags: GlobalFlags, command: Command) -> Self {
        Self { flags, command }
    }
}

#[derive(Clone, Debug, Subcommand)]
pub enum Command {
    #[clap(name = "open", about = "Open project in Android Studio")]
    Open,
    #[clap(name = "check", about = "Checks if code compiles for target(s)")]
    Check {
        #[clap(name = "targets", default_value = Target::DEFAULT_KEY, value_parser = Target::name_list())]
        targets: Vec<String>,
    },
    #[clap(name = "build", about = "Builds dynamic libraries for target(s)")]
    Build {
        #[clap(name = "targets", default_value = Target::DEFAULT_KEY, value_parser = Target::name_list())]
        targets: Vec<String>,
        #[clap(flatten)]
        profile: cli::Profile,
    },
    #[clap(name = "run", about = "Deploys APK to connected device")]
    Run {
        #[clap(flatten)]
        profile: cli::Profile,
        #[clap(flatten)]
        filter: cli::Filter,
        #[clap(flatten)]
        reinstall_deps: cli::ReinstallDeps,
        #[clap(
            short = 'a',
            long = "activity",
            help = "Specifies which activtiy to launch"
        )]
        activity: Option<String>,
        #[clap(
            long = "application-id-suffix",
            help = "Optional suffix for the application ID (e.g. \".debug\")"
        )]
        application_id_suffix: Option<String>,
    },
    #[clap(name = "st", about = "Displays a detailed stacktrace for a device")]
    Stacktrace,
    #[clap(name = "list", about = "Lists connected devices")]
    List,
    #[clap(name = "apk", about = "Manage and build APKs")]
    Apk {
        #[clap(subcommand)]
        cmd: ApkSubcommand,
    },
    #[clap(name = "aab", about = "Manage and build AABs")]
    Aab {
        #[clap(subcommand)]
        cmd: AabSubcommand,
    },
}

#[derive(Subcommand, Clone, Debug)]
pub enum ApkSubcommand {
    #[clap(about = "build APKs (Android Package Kit)")]
    Build {
        #[clap(name = "targets", value_parser = Target::name_list())]
        /// Which targets to build (all by default).
        targets: Vec<String>,
        #[clap(flatten)]
        profile: cli::Profile,
        #[clap(long = "split-per-abi", help = "Whether to split the APKs per ABIs.")]
        split_per_abi: bool,
    },
}
#[derive(Subcommand, Clone, Debug)]
pub enum AabSubcommand {
    #[clap(about = "build AABs (Android App Bundle)")]
    Build {
        #[clap(name = "targets", value_parser = Target::name_list())]
        /// Which targets to build (all by default).
        targets: Vec<String>,
        #[clap(flatten)]
        profile: cli::Profile,
        #[clap(long = "split-per-abi", help = "Whether to split the AABs per ABIs.")]
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
                    "Android Studio project directory {project_dir:?} doesn't exist."
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
                    noise_level: noise_level_count,
                    non_interactive,
                },
            command,
        } = self;
        let noise_level = opts::NoiseLevel::from_occurrences(noise_level_count.into());
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
            Command::Build { targets, profile } => {
                with_config(non_interactive, wrapper, |config, metadata, env| {
                    let profile = profile.profile();
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
                })
            }
            Command::Run {
                profile,
                filter: cli::Filter { filter },
                reinstall_deps: cli::ReinstallDeps { reinstall_deps },
                activity,
                application_id_suffix,
            } => with_config(non_interactive, wrapper, |config, metadata, env| {
                let profile = profile.profile();
                let build_app_bundle = metadata.asset_packs().is_some();
                ensure_init(config)?;
                device_prompt(env)
                    .map_err(Error::DevicePromptFailed)?
                    .run_with_application_id_suffix(
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
                        application_id_suffix,
                    )
                    .and_then(|h| {
                        h.wait().map(|_| ()).map_err(|err| RunError::CommandFailed {
                            command: format!("{h:?}"),
                            error: err,
                        })
                    })
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
                        prompt::list_display_only(
                            device_list
                                .iter()
                                .filter(|d| d.status() == ConnectionStatus::Connected),
                            device_list.len(),
                        );
                    })
            }),
            Command::Apk { cmd } => match cmd {
                ApkSubcommand::Build {
                    targets,
                    profile,
                    split_per_abi,
                } => with_config(non_interactive, wrapper, |config, _, env| {
                    let profile = profile.profile();
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
                    profile,
                    split_per_abi,
                } => with_config(non_interactive, wrapper, |config, _, env| {
                    let profile = profile.profile();
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
