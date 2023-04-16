#![deny(unsafe_code)]

pub mod android;
#[cfg(target_os = "macos")]
pub mod apple;
pub mod bicycle;
pub mod bossy;
pub mod config;
pub mod device;
pub mod doctor;
pub mod dot_cargo;
pub mod env;
pub mod init;
pub mod opts;
pub mod os;
mod project;
mod reserved_names;
pub mod target;
mod templating;
pub mod update;
pub mod util;
use std::ffi::OsStr;

pub use duct::Handle as ChildHandle;

pub static NAME: &str = "mobile";

trait DuctExpressionExt {
    fn vars(self, vars: impl IntoIterator<Item = (impl AsRef<OsStr>, impl AsRef<OsStr>)>) -> Self;
}

impl DuctExpressionExt for duct::Expression {
    fn vars(
        mut self,
        vars: impl IntoIterator<Item = (impl AsRef<OsStr>, impl AsRef<OsStr>)>,
    ) -> Self {
        for (k, v) in vars {
            self = self.env(&k, &v);
        }
        self
    }
}
