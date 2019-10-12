mod adb;
mod config;
mod device;
mod env;
mod exec;
mod ndk;
mod project;
mod target;

use exec::Command;
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
    let app = Command::app(&targets);
    let matches = cli::get_matches(app, NAME).map_err(NonZeroExit::Clap)?;
    let command = Command::parse(matches);
    let umbrella = Umbrella::load(".")
        .map_err(NonZeroExit::display)?
        .ok_or_else(|| NonZeroExit::display("Config not found!"))?;
    let config = umbrella.plugin(NAME).map_err(NonZeroExit::display)?;
    command
        .exec(&config, Default::default())
        .map_err(NonZeroExit::display)
}

fn main() {
    NonZeroExit::main(inner)
}
