#![allow(unsafe_code)]

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use self::macos::*;

#[cfg(any(target_os = "linux", target_os = "android"))]
mod linux;

#[cfg(any(target_os = "linux", target_os = "android"))]
pub use self::linux::*;

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use self::windows::*;

#[cfg(not(any(target_os = "macos", target_os = "android", target_os = "linux", windows)))]
compile_error!("Host platform not yet supported by cargo-mobile! We'd love if you made a PR to add support for this platform ❤️");

// TODO: we should probably expose common functionality throughout `os` in a
// less ad-hoc way... since it's really easy to accidentally break things.
#[derive(Debug)]
pub struct Info {
    pub name: String,
    pub version: String,
}

impl Info {
    pub fn check() -> Result<Self, impl std::error::Error> {
        self::info::check()
    }
}
