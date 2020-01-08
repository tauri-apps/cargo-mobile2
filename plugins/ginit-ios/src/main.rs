#![forbid(unsafe_code)]

mod config;
mod deps;
mod device;
mod exec;
mod ios_deploy;
mod project;
mod system_profile;
mod target;
mod teams;

use ginit_core::{
    config::umbrella::Umbrella,
    target::TargetTrait as _,
    util::cli::{self, NonZeroExit},
};

static NAME: &'static str = "ios";

fn main() {
    NonZeroExit::main(|wrapper| {
        let targets = target::Target::all()
            .keys()
            .map(|key| *key)
            .collect::<Vec<_>>();
        let input =
            cli::get_matches_and_parse(exec::app(&targets), NAME).map_err(NonZeroExit::Clap)?;
        let config = Umbrella::load_plugin(NAME).map_err(NonZeroExit::display)?;
        exec::exec(input, config.as_ref(), wrapper).map_err(NonZeroExit::display)
    })
}
