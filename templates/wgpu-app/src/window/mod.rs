#[cfg(target_os = "android")]
mod el1;
#[cfg(not(target_os = "android"))]
mod el2;

#[cfg(target_os = "android")]
pub use self::el1::*;
#[cfg(not(target_os = "android"))]
pub use self::el2::*;
