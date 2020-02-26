#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use self::macos::*;

#[cfg(not(target_os = "macos"))]
compile_error!("Target platform not yet supported by ginit");

pub static NAME: &'static str = std::env::consts::OS;

pub mod exports {
    pub use into_result;
    #[cfg(target_os = "macos")]
    pub use {cocoa, core_foundation, objc, objc_id};
}
