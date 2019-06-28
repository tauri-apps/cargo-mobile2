mod android;
mod config;
mod init;
mod ios;
mod target;
mod templating;
mod util;

use self::{
    config::{Config, CONFIG},
    templating::init_templating,
};
use std::env;
use structopt::StructOpt;

static NAME: &'static str = "ginit";

#[derive(Debug, StructOpt)]
#[structopt(raw(name = "NAME"), about = "Game dev productivity tools")]
pub struct Args {
    #[structopt(short = "v", long = "verbose", help = "Makes life louder")]
    verbose: bool,
    #[structopt(subcommand)]
    pub subcommand: Subcommand,
}

#[derive(Debug, StructOpt)]
pub enum Subcommand {
    #[structopt(
        name = "init",
        about = "Create a new project in the current working directory"
    )]
    Init {
        #[structopt(long = "force", help = "Clobber files with no remorse")]
        force: bool,
        #[structopt(
            long = "skip",
            help = "Skip some steps",
            raw(possible_values = "init::STEPS")
        )]
        skip: Vec<String>,
    },
    #[structopt(name = "install-deps", about = "Install dependencies for this tool")]
    InstallDeps,
    #[structopt(name = "android", about = "Tools for Android")]
    Android {
        #[structopt(subcommand)]
        subcommand: android::Subcommand,
    },
    #[structopt(name = "ios", about = "Tools for iOS")]
    IOS {
        #[structopt(subcommand)]
        subcommand: ios::Subcommand,
    },
}

fn log_init(verbose: bool) {
    use env_logger::{Builder, Env};
    let default_level = if verbose { "info" } else { "warn" };
    let env = Env::default().default_filter_or(default_level);
    Builder::from_env(env).init();
}

fn parse_args() -> Args {
    let mut raw: Vec<String> = env::args().collect();
    // Running this as a cargo subcommand gives us our name as an argument,
    // so let's just discard that...
    if raw.get(1).map(String::as_str) == Some(NAME) {
        raw.remove(1);
    }
    // We have to initialize logging as early as possible, so we'll do some
    // rough manual parsing.
    let verbose = {
        use crate::util::FriendlyContains;
        // This will fail if multiple short flags are put together, i.e. `-vh`
        raw.friendly_contains("--verbose") || raw.friendly_contains("-v")
    };
    log_init(verbose);
    Args::from_iter(raw)
}

fn main() {
    let args = parse_args();
    match args.subcommand {
        Subcommand::Init { force, skip } => init::init(&init_templating(), force, skip),
        Subcommand::InstallDeps => init::install_deps(),
        Subcommand::Android { subcommand } => subcommand.handle(args.verbose),
        Subcommand::IOS { subcommand } => subcommand.handle(args.verbose),
    }
}
