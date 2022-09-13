#![forbid(unsafe_code)]

#[path = "../android.rs"]
mod android;
#[path = "../cli.rs"]
mod cli;

use android::Input;
use cargo_mobile_lib::android::NAME;

use cli::exec;

fn main() {
    exec::<Input>(NAME)
}
