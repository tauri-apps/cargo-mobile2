#![cfg(target_os = "macos")]
#![forbid(unsafe_code)]

use cargo_mobile::{
    apple::{cli::Input, NAME},
    util::cli::exec,
};

fn main() {
    exec::<Input>(NAME)
}
