#![forbid(unsafe_code)]

use cargo_mobile::{
    android::{cli::Input, NAME},
    util::cli::exec,
};

fn main() {
    exec::<Input>(NAME)
}
