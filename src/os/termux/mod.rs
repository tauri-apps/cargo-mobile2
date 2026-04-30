use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use thiserror::Error;

pub use crate::{
    env::{Env, ExplicitEnv},
    util::ln,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("termux is not installed")]
    NoTermux,
    #[error("termux::API is not installed")]
    NoTermuxApi,
    #[error("IO error")]
    IoError(#[from] std::io::Error),
    #[error("Env error")]
    VarError(#[from] std::env::VarError),
}

#[derive(Debug, Error)]
pub enum DetectEditorError {
    #[error("No default editor is set: xdg-mime queries for \"text/rust\" and \"text/plain\" both failed")]
    NoDefaultEditorSet,
}

#[derive(Debug, Error)]
pub enum OpenFileError {
    #[error("Failed to run {command}: {error}")]
    CommandFailed {
        command: String,
        error: std::io::Error,
    },
    #[error("Command parsing failed")]
    CommandParsingFailed,
    #[error("termux::API is not installed")]
    NoTermuxApi,
}

#[derive(Debug)]
pub struct Application {
    exec_command: OsString,
    // icon: Option<OsString>,
    // xdg_entry_path: PathBuf,
}

impl Application {
    pub fn detect_editor() -> Result<Self, DetectEditorError> {
        let editor = std::env::var("EDITOR").map_err(|_| DetectEditorError::NoDefaultEditorSet)?;
        Ok(Self {
            exec_command: editor.into(),
            // icon: None,
            // xdg_entry_path: PathBuf::new(),
        })
    }

    pub fn open_file(&self, path: impl AsRef<Path>) -> Result<(), OpenFileError> {
        let path = path.as_ref();
        duct::cmd(&self.exec_command, path)
            .run()
            .map(|_| ())
            .map_err(|error| OpenFileError::CommandFailed {
                command: self.exec_command.to_string_lossy().to_string(),
                error,
            })
    }
}

pub fn open_file_with(
    _application: impl AsRef<OsStr>,
    path: impl AsRef<OsStr>,
    _env: &Env,
) -> Result<(), OpenFileError> {
    let _ = termux_api_version().ok_or(OpenFileError::NoTermuxApi)?;
    duct::cmd("termux-open", vec![path.as_ref()])
        .run()
        .map(|_| ())
        .map_err(|error| OpenFileError::CommandFailed {
            command: "termux-open".to_string(),
            error,
        })
}

/// Open file with the $EDITOR (or nano)
pub fn open_with_editor(path: impl AsRef<Path>) -> Result<std::process::Output, Error> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
    Ok(duct::cmd(editor, path.as_ref()).run()?)
}

/// Open file with termux-open (requires termux::API)
pub fn open_with_termux(path: impl AsRef<Path>) -> Result<std::process::Output, Error> {
    let _ = termux_api_version().ok_or(Error::NoTermuxApi);
    Ok(duct::cmd("termux-open", path.as_ref()).run()?)
}

/// Installs an apk by running termux-open, returns an error
/// when termux::API is not installed or termux-open fails.
pub fn run_apk(apk_path: impl AsRef<Path>) -> Result<std::process::Output, Error> {
    open_with_termux(apk_path)
}

/// Returns the `PathBuf` where termux is installed (eg ``/data/data/com.termux/files/usr``)
pub fn prefix() -> Result<PathBuf, Error> {
    let prefix = std::env::var("PREFIX")?;
    Ok(PathBuf::from(prefix))
}

/// Returns `Some(api version)` or None when termux::API version is
/// not available.
pub fn termux_api_version() -> Option<String> {
    std::env::var("TERMUX_API_VERSION").ok()
}

/// Returns `Some(api version)` or None when termux::API version is
/// not available.
pub fn termux_version() -> Option<String> {
    std::env::var("TERMUX_VERSION").ok()
}

pub fn command_path(name: &str) -> std::io::Result<std::process::Output> {
    duct::cmd("sh", ["-c", format!("command -v {name}").as_str()]).run()
}

pub fn code_command() -> duct::Expression {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
    duct::cmd!(editor)
}

pub fn replace_path_separator(path: OsString) -> OsString {
    path
}

pub mod consts {
    pub const CLANG: &str = "clang";
    pub const CLANGXX: &str = "clang++";
    pub const AR: &str = "ar";
    pub const LD: &str = "ld";
    pub const READELF: &str = "readelf";
    pub const NDK_STACK: &str = "ndk-stack";
}

pub mod info {
    use super::Error;
    use crate::os::Info;

    pub fn check() -> Result<Info, Error> {
        Ok(Info {
            name: "termux".to_string(),
            version: super::termux_version().ok_or(Error::NoTermux)?,
        })
    }
}
