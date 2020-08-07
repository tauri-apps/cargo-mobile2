#![forbid(unsafe_code)]

use cargo_mobile::{
    init, opts,
    util::{
        self,
        cli::{self, Exec, GlobalFlags, Report, Reportable, TextWrapper},
    },
    NAME,
};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(bin_name = cli::bin_name(NAME), global_settings = cli::GLOBAL_SETTINGS, settings = cli::SETTINGS)]
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
        reinstall_deps: cli::ReinstallDeps,
        #[structopt(
            long = "open",
            help = "Open in default code editor",
            parse(from_flag = opts::OpenInEditor::from_bool),
        )]
        open_in_editor: opts::OpenInEditor,
        #[structopt(
            long,
            help = "Only do some steps",
            value_name = "STEPS",
            possible_values = init::STEPS,
            value_delimiter(" "),
        )]
        only: Option<Vec<String>>,
        #[structopt(
            long,
            help = "Skip some steps",
            value_name = "STEPS",
            possible_values = init::STEPS,
            value_delimiter(" "),
        )]
        skip: Option<Vec<String>>,
    },
    #[structopt(name = "open", about = "Open project in default code editor")]
    Open,
    #[structopt(name = "update", about = "Update `cargo-mobile`")]
    Update {
        #[structopt(long = "init", help = "Regenerate project if update succeeds")]
        init: bool,
    },
}

#[derive(Debug)]
pub enum Error {
    InitFailed(init::Error),
    OpenFailed(util::OpenInEditorError),
    UpdateFailed(bossy::Error),
}

impl Reportable for Error {
    fn report(&self) -> Report {
        match self {
            Self::InitFailed(err) => err.report(),
            Self::OpenFailed(err) => {
                Report::error("Failed to open project in default code editor", err)
            }
            Self::UpdateFailed(err) => Report::error("Failed to update `cargo-mobile`", err),
        }
    }
}

impl Exec for Input {
    type Report = Error;

    fn global_flags(&self) -> GlobalFlags {
        self.flags
    }

    fn exec(self, wrapper: &TextWrapper) -> Result<(), Self::Report> {
        let Self {
            flags: GlobalFlags {
                non_interactive, ..
            },
            command,
        } = self;
        match command {
            Command::Init {
                skip_dev_tools: cli::SkipDevTools { skip_dev_tools },
                reinstall_deps: cli::ReinstallDeps { reinstall_deps },
                open_in_editor,
                only,
                skip,
            } => init::exec(
                wrapper,
                non_interactive,
                skip_dev_tools,
                reinstall_deps,
                open_in_editor,
                only,
                skip,
                ".",
            )
            .map(|_| ())
            .map_err(Error::InitFailed),
            Command::Open => util::open_in_editor(".").map_err(Error::OpenFailed),
            Command::Update { init } => {
                bossy::Command::impure("cargo")
                    .with_args(&[
                        "install",
                        "--force",
                        "--git",
                        "https://github.com/BrainiumLLC/cargo-mobile",
                    ])
                    .run_and_wait()
                    .map(|_| ())
                    .map_err(Error::UpdateFailed)?;
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
                    .map_err(Error::InitFailed)?;
                }
                Ok(())
            }
        }
    }
}

fn main() {
    cli::exec::<Input>(NAME)
}
