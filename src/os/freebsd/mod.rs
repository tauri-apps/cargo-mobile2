pub(super) mod info;

#[path = "../linux/xdg.rs"]
mod xdg;

use std::{
    ffi::{OsStr, OsString},
    io,
    path::{Path, PathBuf},
};
use thiserror::Error;

use crate::DuctExpressionExt;
pub use crate::{
    env::{Env, ExplicitEnv},
    util::ln,
};

#[derive(Debug, Error)]
pub enum DetectEditorError {
    #[error("No default editor is set: xdg-mime queries for \"text/rust\" and \"text/plain\" both failed")]
    NoDefaultEditorSet,
    #[error("Entry Not Found: xdg-mime returned an entry name that could not be found")]
    FreeDesktopEntryNotFound,
    #[error(
        "Entry Parse Error: xdg-mime returned an entry that could not be parsed. Caused by {0}"
    )]
    FreeDesktopEntryParseError(io::Error),
    #[error("Entry Parse Error: file lookup failed. Caused by {0}")]
    FreeDesktopEntryLookupFailed(io::Error),
    #[error("Exec field on desktop entry was not found")]
    ExecFieldMissing,
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
}

#[derive(Debug)]
pub struct Application {
    exec_command: OsString,
    icon: Option<OsString>,
    xdg_entry_path: PathBuf,
}

impl Application {
    pub fn detect_editor() -> Result<Self, DetectEditorError> {
        let entry = xdg::query_mime_entry("text/rust")
            .or_else(|| xdg::query_mime_entry("text/plain"))
            .ok_or(DetectEditorError::NoDefaultEditorSet)?;

        xdg::get_xdg_data_dirs()
            .iter()
            .find_map(|dir| {
                let dir = dir.join("applications");
                xdg::find_entry_in_dir(&dir, &entry)
                    .ok()?
                    .map(|entry_filepath| {
                        xdg::parse(&entry_filepath)
                            .map_err(DetectEditorError::FreeDesktopEntryParseError)
                            .and_then(|parsed_entry| {
                                Ok(Self {
                                    exec_command: parsed_entry
                                        .section("Desktop Entry")
                                        .and_then(|s| s.attr("Exec").first())
                                        .ok_or(DetectEditorError::ExecFieldMissing)?
                                        .into(),
                                    icon: parsed_entry
                                        .section("Desktop Entry")
                                        .and_then(|s| s.attr("Icon").first())
                                        .map(Into::into),
                                    xdg_entry_path: entry_filepath,
                                })
                            })
                    })
            })
            .unwrap_or(Err(DetectEditorError::FreeDesktopEntryNotFound))
    }

    pub fn open_file(&self, path: impl AsRef<Path>) -> Result<(), OpenFileError> {
        let path = path.as_ref();
        let maybe_icon = self.icon.as_deref();

        let command_parts = xdg::parse_command(
            &self.exec_command,
            path.as_os_str(),
            maybe_icon,
            Some(&self.xdg_entry_path),
        );

        if !command_parts.is_empty() {
            let cmd = duct::cmd(&command_parts[0], &command_parts[1..]);
            cmd.run_and_detach()
                .map_err(|error| OpenFileError::CommandFailed {
                    command: format!("{cmd:?}"),
                    error,
                })
        } else {
            Err(OpenFileError::CommandParsingFailed)
        }
    }
}

pub fn open_file_with(
    application: impl AsRef<OsStr>,
    path: impl AsRef<OsStr>,
    env: &Env,
) -> Result<(), OpenFileError> {
    let app_str = application.as_ref();
    let path_str = path.as_ref();

    let command_parts = xdg::get_xdg_data_dirs()
        .iter()
        .find_map(|dir| {
            let dir = dir.join("applications");
            let (entry, entry_path) = xdg::find_entry_by_app_name(&dir, app_str)?;

            let command_parts = entry
                .section("Desktop Entry")
                .and_then(|s| s.attr("Exec").first())
                .map(|str_entry| {
                    xdg::parse_command(
                        str_entry.as_ref(),
                        path_str,
                        entry
                            .section("Desktop Entry")
                            .and_then(|s| s.attr("Icon").first())
                            .map(|s| s.as_ref()),
                        Some(&entry_path),
                    )
                })?;
            if !command_parts.is_empty() {
                Some(command_parts)
            } else {
                None
            }
        })
        .unwrap_or_else(|| vec![app_str.to_os_string()]);

    let cmd = duct::cmd(&command_parts[0], &command_parts[1..]).vars(env.explicit_env());
    cmd.run_and_detach()
        .map_err(|error| OpenFileError::CommandFailed {
            command: format!("{cmd:?}"),
            error,
        })
}

#[cfg(target_os = "freebsd")]
pub fn command_path(name: &str) -> std::io::Result<std::process::Output> {
    duct::cmd("sh", ["-c", format!("command -v {name}").as_str()]).run()
}

pub fn code_command() -> duct::Expression {
    duct::cmd!("code")
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
