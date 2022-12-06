pub mod aab;
pub mod adb;
pub mod apk;
mod bundletool;
#[cfg(feature = "cli")]
pub mod cli;
pub mod config;
pub mod device;
pub mod emulator;
pub mod env;
mod jnilibs;
pub mod ndk;
pub(crate) mod project;
mod source_props;
pub mod target;

pub static NAME: &str = "android";
pub static DEFAULT_ACTIVITY: &str = "android.app.NativeActivity";
pub static DEFAULT_THEME_PARENT: &str = "android:Theme.Material.Light.DarkActionBar";
