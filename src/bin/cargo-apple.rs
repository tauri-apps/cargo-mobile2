#![forbid(unsafe_code)]

use cargo_mobile::{
    apple::{
        config::{Config, Metadata},
        device::{Device, RunError},
        ios_deploy,
        target::{ArchiveError, BuildError, CheckError, CompileLibError, ExportError, Target},
        NAME,
    },
    config::{
        metadata::{self, Metadata as OmniMetadata},
        Config as OmniConfig, LoadOrGenError,
    },
    define_device_prompt,
    device::PromptError,
    env::{Env, Error as EnvError},
    init, opts, os,
    target::{call_for_targets_with_fallback, TargetInvalid, TargetTrait as _},
    util::{
        cli::{self, Exec, GlobalFlags, Report, Reportable, TextWrapper},
        prompt,
    },
};
use structopt::{clap::AppSettings, StructOpt};

#[derive(Debug, StructOpt)]
#[structopt(bin_name = cli::bin_name(NAME), global_settings = cli::GLOBAL_SETTINGS, settings = cli::SETTINGS)]
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
        skip_dev_tools: cli::SkipDevTools,
        #[structopt(flatten)]
        reinstall_deps: cli::ReinstallDeps,
        #[structopt(
            long = "open",
            help = "Open in Xcode",
            parse(from_flag = opts::OpenInEditor::from_bool),
        )]
        open_in_editor: opts::OpenInEditor,
    },
    #[structopt(name = "open", about = "Open project in Xcode")]
    Open,
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
    #[structopt(name = "archive", about = "Builds and archives for targets(s)")]
    Archive {
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
        #[structopt(long = "macos", help = "Awkwardly special-case for macOS")]
        macos: bool,
        #[structopt(name = "ARCH", index = 1, required = true)]
        arch: String,
        #[structopt(flatten)]
        profile: cli::Profile,
        #[structopt(
            long = "force-color",
            help = "Force colorization of output",
            hidden = true,
            parse(from_flag = opts::ForceColor::from_bool),
        )]
        force_color: opts::ForceColor,
    },
}

#[derive(Debug)]
pub enum Error {
    EnvInitFailed(EnvError),
    DevicePromptFailed(PromptError<ios_deploy::DeviceListError>),
    TargetInvalid(TargetInvalid),
    ConfigFailed(LoadOrGenError),
    MetadataFailed(metadata::Error),
    InitFailed(init::Error),
    OpenFailed(bossy::Error),
    CheckFailed(CheckError),
    BuildFailed(BuildError),
    ArchiveFailed(ArchiveError),
    ExportFailed(ExportError),
    RunFailed(RunError),
    ListFailed(ios_deploy::DeviceListError),
    ArchInvalid { arch: String },
    CompileLibFailed(CompileLibError),
}

impl Reportable for Error {
    fn report(&self) -> Report {
        match self {
            Self::EnvInitFailed(err) => err.report(),
            Self::DevicePromptFailed(err) => err.report(),
            Self::TargetInvalid(err) => Report::error("Specified target was invalid", err),
            Self::ConfigFailed(err) => err.report(),
            Self::MetadataFailed(err) => err.report(),
            Self::InitFailed(err) => err.report(),
            Self::OpenFailed(err) => Report::error("Failed to open project in Xcode", err),
            Self::CheckFailed(err) => err.report(),
            Self::BuildFailed(err) => err.report(),
            Self::ArchiveFailed(err) => err.report(),
            Self::ExportFailed(err) => err.report(),
            Self::RunFailed(err) => err.report(),
            Self::ListFailed(err) => err.report(),
            Self::ArchInvalid { arch } => Report::error(
                "`cargo-xcode.sh` bug",
                format!("Specified arch {:?} was invalid", arch),
            ),
            Self::CompileLibFailed(err) => err.report(),
        }
    }
}

impl Exec for Input {
    type Report = Error;

    fn global_flags(&self) -> GlobalFlags {
        self.flags
    }

    fn exec(self, wrapper: &TextWrapper) -> Result<(), Self::Report> {
        define_device_prompt!(ios_deploy::device_list, ios_deploy::DeviceListError, iOS);
        fn detect_target_ok<'a>(env: &Env) -> Option<&'a Target<'a>> {
            device_prompt(env).map(|device| device.target()).ok()
        }

        fn with_config(
            non_interactive: opts::NonInteractive,
            wrapper: &TextWrapper,
            f: impl FnOnce(&Config) -> Result<(), Error>,
        ) -> Result<(), Error> {
            let (config, _origin) = OmniConfig::load_or_gen(".", non_interactive, wrapper)
                .map_err(Error::ConfigFailed)?;
            f(config.apple())
        }

        fn with_config_and_metadata(
            non_interactive: opts::NonInteractive,
            wrapper: &TextWrapper,
            f: impl FnOnce(&Config, &Metadata) -> Result<(), Error>,
        ) -> Result<(), Error> {
            with_config(non_interactive, wrapper, |config| {
                let metadata =
                    OmniMetadata::load(&config.app().root_dir()).map_err(Error::MetadataFailed)?;
                f(config, &metadata.apple)
            })
        }

        fn open_in_xcode(config: &Config) -> Result<(), Error> {
            os::open_file_with("Xcode", config.project_dir()).map_err(Error::OpenFailed)
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
            Command::Init {
                skip_dev_tools: cli::SkipDevTools { skip_dev_tools },
                reinstall_deps: cli::ReinstallDeps { reinstall_deps },
                open_in_editor,
            } => {
                let config = init::exec(
                    wrapper,
                    non_interactive,
                    skip_dev_tools,
                    reinstall_deps,
                    Default::default(),
                    Some(vec!["apple".into()]),
                    None,
                    ".",
                )
                .map_err(Error::InitFailed)?;
                if open_in_editor.yes() {
                    open_in_xcode(config.apple())
                } else {
                    Ok(())
                }
            }
            Command::Open => with_config(non_interactive, wrapper, open_in_xcode),
            Command::Check { targets } => {
                with_config_and_metadata(non_interactive, wrapper, |config, metadata| {
                    call_for_targets_with_fallback(
                        targets.iter(),
                        &detect_target_ok,
                        &env,
                        |target: &Target| {
                            target
                                .check(config, metadata, &env, noise_level)
                                .map_err(Error::CheckFailed)
                        },
                    )
                    .map_err(Error::TargetInvalid)?
                })
            }
            Command::Build {
                targets,
                profile: cli::Profile { profile },
            } => with_config(non_interactive, wrapper, |config| {
                call_for_targets_with_fallback(
                    targets.iter(),
                    &detect_target_ok,
                    &env,
                    |target: &Target| {
                        target
                            .build(config, &env, noise_level, profile)
                            .map_err(Error::BuildFailed)
                    },
                )
                .map_err(Error::TargetInvalid)?
            }),
            Command::Archive {
                targets,
                profile: cli::Profile { profile },
            } => with_config(non_interactive, wrapper, |config| {
                call_for_targets_with_fallback(
                    targets.iter(),
                    &detect_target_ok,
                    &env,
                    |target: &Target| {
                        target
                            .build(config, &env, noise_level, profile)
                            .map_err(Error::BuildFailed)?;
                        target
                            .archive(config, &env, noise_level, profile)
                            .map_err(Error::ArchiveFailed)
                    },
                )
                .map_err(Error::TargetInvalid)?
            }),
            Command::Run {
                profile: cli::Profile { profile },
            } => with_config(non_interactive, wrapper, |config| {
                device_prompt(&env)
                    .map_err(Error::DevicePromptFailed)?
                    .run(config, &env, wrapper, noise_level, non_interactive, profile)
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
                force_color,
            } => with_config_and_metadata(non_interactive, wrapper, |config, metadata| {
                match macos {
                    true => Target::macos().compile_lib(
                        config,
                        metadata,
                        noise_level,
                        force_color,
                        profile,
                    ),
                    false => Target::for_arch(&arch)
                        .ok_or_else(|| Error::ArchInvalid {
                            arch: arch.to_owned(),
                        })?
                        .compile_lib(config, metadata, noise_level, force_color, profile),
                }
                .map_err(Error::CompileLibFailed)
            }),
        }
    }
}

fn main() {
    cli::exec::<Input>(NAME)
}
