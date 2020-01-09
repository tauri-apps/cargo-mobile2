#![forbid(unsafe_code)]

mod adb;
mod config;
mod device;
mod env;
mod exec;
mod ndk;
mod project;
mod target;

use self::exec::Input;
use ginit_core::{
    config::umbrella::Umbrella,
    util::cli::{self, NonZeroExit},
};
use structopt::StructOpt as _;

static NAME: &'static str = "android";

fn main() {
    NonZeroExit::main(|wrapper| {
        let input = Input::from_iter_safe(cli::get_args(NAME)).map_err(NonZeroExit::Clap)?;
        let config = Umbrella::load_plugin(NAME).map_err(NonZeroExit::display)?;
        input
            .exec(config.as_ref(), wrapper)
            .map_err(NonZeroExit::display)
    })
}
