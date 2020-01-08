#![forbid(unsafe_code)]

mod adb;
mod config;
mod device;
mod env;
mod exec;
mod ndk;
mod project;
mod target;

use ginit_core::{
    config::umbrella::Umbrella,
    target::TargetTrait as _,
    util::cli::{self, NonZeroExit},
};

static NAME: &'static str = "android";

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
