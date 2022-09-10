#![forbid(unsafe_code)]

#[cfg(target_os = "macos")]
#[path = "../apple.rs"]
mod apple;
#[path = "../cli.rs"]
mod cli;

#[cfg(target_os = "macos")]
fn main() {
    use apple::Input;
    use cargo_mobile_core::apple::NAME;
    use cli::exec;
    exec::<Input>(NAME)
}

#[cfg(not(target_os = "macos"))]
fn main() {
    use cli::{Exit, Report};
    Exit::main(|_wrapper| {
        Err(Exit::Report(Report::error(
            "`cargo-apple` can only be used on macOS",
            "Apple doesn't support building for iOS on other platforms, sorry!",
        )))
    })
}
