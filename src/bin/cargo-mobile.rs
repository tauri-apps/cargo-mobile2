#![forbid(unsafe_code)]

use cargo_mobile::{
    init, opts,
    util::{
        self,
        cli::{self, Exec, ExecError, GlobalFlags, TextWrapper},
    },
    NAME,
};
use std::fmt::{self, Display};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(bin_name = cli::bin_name(NAME), settings = cli::SETTINGS)]
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
        clobbering: cli::Clobbering,
        #[structopt(
            long,
            about = "Open in default code editor",
            parse(from_flag = opts::OpenIn::from_flag),
        )]
        open: opts::OpenIn,
        #[structopt(
            long,
            about = "Only do some steps",
            value_name = "STEPS",
            possible_values = init::STEPS,
            value_delimiter(" "),
        )]
        only: Option<Vec<String>>,
        #[structopt(
            long,
            about = "Skip some steps",
            value_name = "STEPS",
            possible_values = init::STEPS,
            value_delimiter(" "),
        )]
        skip: Option<Vec<String>>,
    },
    #[structopt(name = "open", about = "Open project in default code editor")]
    Open,
}

#[derive(Debug)]
pub enum Error {
    ConfigMissing,
    InitFailed(init::Error),
    OpenFailed(util::OpenInEditorError),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigMissing => write!(f, "Config still absent after init"),
            Self::InitFailed(err) => write!(f, "Failed to generate project: {}", err),
            Self::OpenFailed(err) => {
                write!(f, "Failed to open project in default code editor: {}", err)
            }
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
        let Self {
            flags: GlobalFlags { interactivity, .. },
            command,
        } = self;
        match command {
            Command::Init {
                clobbering: cli::Clobbering { clobbering },
                open,
                only,
                skip,
            } => init::exec(interactivity, clobbering, open, only, skip, wrapper)
                .map(|_| ())
                .map_err(Error::InitFailed),
            Command::Open => util::open_in_editor(".").map_err(Error::OpenFailed),
        }
    }
}

fn main() {
    cli::exec::<Input>(NAME)
}
