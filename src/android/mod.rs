mod config;
pub mod project;
mod target;

pub use self::{config::Config, target::Target};
use self::target::POSSIBLE_TARGETS;
use crate::{target::{call_for_targets, FallbackBehavior, TargetTrait}};
use std::convert::identity;
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
    #[structopt(name = "build", about = "Builds shared libraries for target(s)")]
    Build {
        #[structopt(name = "targets", raw(possible_values = "POSSIBLE_TARGETS.as_slice()"))]
        targets: Vec<String>,
        #[structopt(long = "release")]
        release: bool,
    },
    #[structopt(name = "run", about = "Deploy APK for target(s)")]
    Run {
        #[structopt(name = "targets", raw(possible_values = "POSSIBLE_TARGETS.as_slice()"))]
        targets: Vec<String>,
        #[structopt(long = "release")]
        release: bool,
    },
    #[structopt(name = "st", about = "Displays a detailed stacktrace for a target")]
    Stacktrace {
        #[structopt(name = "target", raw(possible_values = "POSSIBLE_TARGETS.as_slice()"))]
        target: Option<String>,
    }
}

fn detect_target() -> Option<&'static Target> {
    let target = Target::for_connected().ok().and_then(identity);
    if let Some(target) = target {
        println!("Detected target for connected device: {}", target.triple);
    }
    target
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
                FallbackBehavior::get_target(&detect_target, true),
                |target: &Target| target.check(verbose),
            ),
            Subcommand::Build { targets, release } => call_for_targets(
                Some(targets),
                FallbackBehavior::get_target(&detect_target, true),
                |target: &Target| target.build(verbose, release),
            ),
            Subcommand::Run { targets, release } => call_for_targets(
                Some(targets),
                FallbackBehavior::get_target(&detect_target, true),
                |target: &Target| target.run(verbose, release),
            ),
            Subcommand::Stacktrace { target } => call_for_targets(
                target.map(|target| vec![target]),
                FallbackBehavior::get_target(&detect_target, false),
                |target: &Target| target.stacktrace(),
            ),
        }
    }
}
