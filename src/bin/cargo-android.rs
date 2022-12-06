#![forbid(unsafe_code)]

use tauri_mobile::{
    android::{cli::Input, NAME},
    util::cli::exec,
};

fn main() {
    exec::<Input>(NAME)
}
