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
    util::{
        cli::{self, NonZeroExit},
        TextWrapper,
    },
};

static NAME: &'static str = "android";

fn inner(_wrapper: &TextWrapper) -> Result<(), NonZeroExit> {
    let targets = target::Target::all()
        .keys()
        .map(|key| *key)
        .collect::<Vec<_>>();
    let input = cli::get_matches_and_parse(exec::app(&targets), NAME).map_err(NonZeroExit::Clap)?;
    let config = Umbrella::load_plugin(".", NAME).map_err(NonZeroExit::display)?;
    exec::exec(input, &config).map_err(NonZeroExit::display)
}

fn main() {
    NonZeroExit::main(inner)
}
