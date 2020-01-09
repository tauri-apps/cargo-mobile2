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
use ginit_core::util::cli::NonZeroExit;

static NAME: &'static str = "android";

fn main() {
    NonZeroExit::exec::<Input>(NAME)
}
