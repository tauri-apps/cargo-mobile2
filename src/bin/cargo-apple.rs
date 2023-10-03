#![cfg(feature = "cli")]
#![forbid(unsafe_code)]

#[cfg(target_os = "macos")]
fn main() {
    use cargo_mobile2::{
        apple::{cli::Input, NAME},
        util::cli::exec,
    };
    exec::<Input>(NAME)
}

#[cfg(not(target_os = "macos"))]
fn main() {
    use cargo_mobile2::util::cli::{Exit, Report};
    Exit::main(|_wrapper| {
        Err(Exit::Report(Report::error(
            "`cargo-apple` can only be used on macOS",
            "Apple doesn't support building for iOS on other platforms, sorry!",
        )))
    })
}
