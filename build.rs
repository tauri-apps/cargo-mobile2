// use std::{env, path::Path, process::Command};

// macro_rules! runtime_env {
//     ($name:expr) => {
//         env::var($name).expect("Missing env var `$name`")
//     };
// }

// fn cargo_prefix(name: String) -> String {
//     match name.starts_with("cargo-") {
//         true => name,
//         false => format!("cargo-{}", name),
//     }
// }

// Install so it can be used as a cargo subcommand
// This is commented out, since you should generally be using `cargo install`.
fn main() {
    // let name = runtime_env!("CARGO_PKG_NAME");
    // let src = Path::new(&runtime_env!("OUT_DIR"))
    //     .join("../../..")
    //     .canonicalize()
    //     .expect("Invalid `CARGO_MANIFEST_DIR`")
    //     .join(&name);
    // let dest = Path::new(&concat!(env!("HOME"), "/.cargo/bin"))
    //     .canonicalize()
    //     .expect("`~/.cargo/bin` couldn't be found")
    //     .join(cargo_prefix(name));
    // Command::new("ln")
    //     .arg("-sf") // always recreate the symlink
    //     .arg(src)
    //     .arg(dest)
    //     .status()
    //     .expect("Failed to symlink binary");
}
