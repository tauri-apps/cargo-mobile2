#![allow(unsafe_code)]

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use self::macos::*;

#[cfg(not(target_os = "macos"))]
compile_error!("Host platform not yet supported by cargo-mobile! We'd love if you made a PR to add support for this platform ❤️");
