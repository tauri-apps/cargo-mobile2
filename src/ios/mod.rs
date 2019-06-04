mod config;
pub mod project;
mod target;
mod teams;

pub use self::{config::Config, target::Target, teams::*};
use self::target::{MACOS, POSSIBLE_TARGETS};
use crate::target::{call_for_targets, FallbackBehavior, TargetTrait};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub enum Subcommand {
    #[structopt(name = "toolchain-init", about = "Installs Rust toolchain for target(s)")]
    ToolchainInit {
        #[structopt(name = "targets", raw(possible_values = "POSSIBLE_TARGETS.as_slice()"))]
        targets: Vec<String>,
    },
    #[structopt(name = "check", about = "Checks if code compiles for target(s)")]
    Check {
        #[structopt(name = "targets", raw(possible_values = "POSSIBLE_TARGETS.as_slice()"))]
        targets: Vec<String>,
    },
    #[structopt(name = "build")]
    Build {
        #[structopt(long = "release")]
        release: bool,
    },
    #[structopt(name = "run")]
    Run {
        #[structopt(long = "release")]
        release: bool,
    },
    #[structopt(name = "compile-lib", about = "Compile static lib (should only be called by Xcode!)")]
    CompileLib {
        #[structopt(long = "macos", help = "Awkwardly special-case for macOS")]
        macos: bool,
        #[structopt(name = "arch")]
        arch: String,
        #[structopt(long = "release")]
        release: bool,
    },
}

impl Subcommand {
    pub fn handle(self, verbose: bool) {
        match self {
            Subcommand::ToolchainInit { targets } => call_for_targets(
                Some(targets),
                FallbackBehavior::all_targets(),
                |target: &Target| target.rustup_add(),
            ),
            Subcommand::Check { targets } => call_for_targets(
                Some(targets),
                FallbackBehavior::all_targets(),
                |target: &Target| target.check(verbose),
            ),
            Subcommand::Build { release } => Target::build(release),
            Subcommand::Run { release } => Target::run(release),
            Subcommand::CompileLib { macos, arch, release } => match macos {
                true => &*MACOS,
                false => Target::for_arch(&arch).expect("Invalid architecture"),
            }.compile_lib(verbose, release),
        }
    }
}
