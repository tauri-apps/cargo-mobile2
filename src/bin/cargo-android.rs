#![cfg(feature = "cli")]
#![forbid(unsafe_code)]

use cargo_mobile2::{
    android::{cli::Input, NAME},
    util::cli::exec,
};

fn main() {
    exec::<Input>(NAME)
}
