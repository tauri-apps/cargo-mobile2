#![cfg(feature = "cli")]
#![forbid(unsafe_code)]

use cargo_mobile2::{
    doctor, init, update,
    util::{
        self,
        cli::{
            self, Exec, GlobalFlags, Report, Reportable, TextWrapper, VERSION_LONG, VERSION_SHORT,
        },
    },
    NAME,
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

#[derive(Clone, Debug, StructOpt)]
pub enum Command {
    #[structopt(
        name = "init",
        about = "Creates a new project in the current working directory"
    )]
    Init {
        #[structopt(flatten)]
        skip_dev_tools: cli::SkipDevTools,
        #[structopt(flatten)]
        skip_targets_install: cli::SkipTargetsInstall,
        #[structopt(flatten)]
        reinstall_deps: cli::ReinstallDeps,
        #[structopt(long = "open", help = "Open in default code editor")]
        open_in_editor: bool,
        #[structopt(long = "submodule-commit", help = "Template pack commit to checkout")]
        submodule_commit: Option<String>,
    },
    #[structopt(name = "new", about = "Creates a new project in a new directory")]
    New {
        #[structopt(flatten)]
        skip_dev_tools: cli::SkipDevTools,
        #[structopt(flatten)]
        skip_targets_install: cli::SkipTargetsInstall,
        #[structopt(flatten)]
        reinstall_deps: cli::ReinstallDeps,
        #[structopt(long = "open", help = "Open in default code editor")]
        open_in_editor: bool,
        #[structopt(long = "submodule-commit", help = "Template pack commit to checkout")]
        submodule_commit: Option<String>,
        #[structopt(
            name = "DIRECTORY",
            help = "New directory to create project in",
            index = 1,
            required = true
        )]
        directory: PathBuf,
    },
    #[structopt(name = "open", about = "Open project in default code editor")]
    Open,
    #[structopt(name = "update", about = "Update `cargo-mobile2`")]
    Update {
        #[structopt(long = "init", help = "Regenerate project if update succeeds")]
        init: bool,
    },
    #[cfg_attr(
        target_os = "macos",
        structopt(
            name = "apple",
            about = "iOS commands (tip: type less by running `cargo apple` instead!)"
        )
    )]
    #[cfg(target_os = "macos")]
    Apple(cargo_mobile2::apple::cli::Command),
    #[structopt(
        name = "android",
        about = "Android commands (tip: type less by running `cargo android` instead!)"
    )]
    Android(cargo_mobile2::android::cli::Command),
    #[structopt(
        name = "doctor",
        about = "Perform a check-up on your installation and environment"
    )]
    Doctor,
}

#[derive(Debug)]
pub enum Error {
    InitFailed(init::Error),
    DirCreationFailed {
        path: PathBuf,
        source: std::io::Error,
    },
    DirChangeFailed {
        path: PathBuf,
        source: std::io::Error,
    },
    OpenFailed(util::OpenInEditorError),
    UpdateFailed(update::Error),
    #[cfg(target_os = "macos")]
    AppleFailed(cargo_mobile2::apple::cli::Error),
    AndroidFailed(cargo_mobile2::android::cli::Error),
    DoctorFailed(doctor::Unrecoverable),
}

impl Reportable for Error {
    fn report(&self) -> Report {
        match self {
            Self::InitFailed(err) => err.report(),
            Self::DirCreationFailed { path, source } => {
                Report::error(format!("Failed to create directory {:?}", path), source)
            }
            Self::DirChangeFailed { path, source } => Report::error(
                format!("Failed to change current directory {:?}", path),
                source,
            ),
            Self::OpenFailed(err) => {
                Report::error("Failed to open project in default code editor", err)
            }
            Self::UpdateFailed(err) => Report::error("Failed to update `cargo-mobile2`", err),
            #[cfg(target_os = "macos")]
            Self::AppleFailed(err) => err.report(),
            Self::AndroidFailed(err) => err.report(),
            Self::DoctorFailed(err) => Report::error("Failed to run doctor", err),
        }
    }
}

impl Exec for Input {
    type Report = Error;

    fn global_flags(&self) -> GlobalFlags {
        self.flags
    }

    fn exec(self, wrapper: &TextWrapper) -> Result<(), Self::Report> {
        let Self { flags, command } = self;
        let GlobalFlags {
            non_interactive, ..
        } = flags;
        match command {
            Command::Init {
                skip_dev_tools: cli::SkipDevTools { skip_dev_tools },
                skip_targets_install:
                    cli::SkipTargetsInstall {
                        skip_targets_install,
                    },
                reinstall_deps: cli::ReinstallDeps { reinstall_deps },
                open_in_editor,
                submodule_commit,
            } => init::exec(
                wrapper,
                non_interactive,
                skip_dev_tools,
                skip_targets_install,
                reinstall_deps,
                open_in_editor,
                submodule_commit,
                ".",
            )
            .map(|_| ())
            .map_err(|e| Error::InitFailed(*e)),
            Command::New {
                skip_dev_tools: cli::SkipDevTools { skip_dev_tools },
                skip_targets_install:
                    cli::SkipTargetsInstall {
                        skip_targets_install,
                    },
                reinstall_deps: cli::ReinstallDeps { reinstall_deps },
                open_in_editor,
                submodule_commit,
                directory,
            } => {
                std::fs::create_dir_all(&directory).map_err(|source| Error::DirCreationFailed {
                    path: directory.clone(),
                    source,
                })?;
                std::env::set_current_dir(&directory).map_err(|source| Error::DirChangeFailed {
                    path: directory,
                    source,
                })?;
                init::exec(
                    wrapper,
                    non_interactive,
                    skip_dev_tools,
                    skip_targets_install,
                    reinstall_deps,
                    open_in_editor,
                    submodule_commit,
                    ".",
                )
                .map(|_| ())
                .map_err(|e| Error::InitFailed(*e))
            }
            Command::Open => util::open_in_editor(".").map_err(Error::OpenFailed),
            Command::Update { init } => {
                update::update(wrapper).map_err(Error::UpdateFailed)?;
                if init {
                    init::exec(
                        wrapper,
                        non_interactive,
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        ".",
                    )
                    .map_err(|e| Error::InitFailed(*e))?;
                }
                Ok(())
            }
            #[cfg(target_os = "macos")]
            Command::Apple(command) => cargo_mobile2::apple::cli::Input::new(flags, command)
                .exec(wrapper)
                .map_err(Error::AppleFailed),
            Command::Android(command) => cargo_mobile2::android::cli::Input::new(flags, command)
                .exec(wrapper)
                .map_err(Error::AndroidFailed),
            Command::Doctor => doctor::exec(wrapper).map_err(Error::DoctorFailed),
        }
    }
}

fn main() {
    cli::exec::<Input>(NAME)
}
