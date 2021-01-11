mod xdg;

use std::{
    ffi::{OsStr, OsString},
    fmt::{self, Display},
    io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum DetectEditorError {
    NoDefaultEditorSet,
    FreeDesktopEntryNotFound,
    FreeDesktopEntryParseError(io::Error),
    FreeDesktopEntryLookupFailed(io::Error),
    ExecFieldMissing,
}

impl Display for DetectEditorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoDefaultEditorSet => write!(f, "No default editor is set: xdg-mime queries for \"text/rust\" and \"text/plain\" both failed"),
            Self::FreeDesktopEntryNotFound => write!(f, "Entry Not Found: xdg-mime returned an entry name that could not be found"),
            Self::FreeDesktopEntryParseError(e) => write!(f, "Entry Parse Error: xdg-mime returned an entry that could not be parsed. Caused by {:?}", e),
            Self::FreeDesktopEntryLookupFailed(e) => write!(f, "Entry Parse Error: file lookup failed. Caused by {:?}", e),
            Self::ExecFieldMissing => write!(f, "Exec field on desktop entry was not found"),
        }
    }
}

#[derive(Debug)]
pub enum OpenFileError {
    LaunchFailed(bossy::Error),
    CommandParsingFailed,
}

impl Display for OpenFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LaunchFailed(e) => write!(f, "Launch failed: {}", e),
            Self::CommandParsingFailed => write!(f, "Command parsing failed"),
        }
    }
}

#[derive(Debug)]
pub struct Application {
    exec_command: OsString,
    icon: Option<OsString>,
    xdg_entry_path: PathBuf,
}

impl Application {
    pub fn detect_editor() -> Result<Self, DetectEditorError> {
        // Try a rust code editor, then a plain text editor. If neither are available,
        // then return an error.
        let entry = xdg::query_mime_entry("text/rust")
            .or_else(|| xdg::query_mime_entry("text/plain"))
            .ok_or(DetectEditorError::NoDefaultEditorSet)?;

        xdg::get_xdg_data_dirs()
            .iter()
            .find_map(|dir| {
                let dir = dir.join("applications");
                xdg::find_entry_in_dir(&dir, &entry)
                    // If finding an entry (a filename) in that directory, returns an error (such as if the directory
                    // is non existent, or the directory exists but for whatever reason listing its contents failed),
                    // we should skip it, as per the XDG Base Directory Specification v0.7 (latest as of today)
                    .ok()? // This returns None on error, continuing the search (skiping this dir)
                    .map(|entry_filepath| {
                        // If something was found, we have to try parsing it, which may fail as well
                        xdg::parse(&entry_filepath)
                            .map_err(DetectEditorError::FreeDesktopEntryParseError)
                            .and_then(|parsed_entry| {
                                Ok(Self {
                                    // We absolutely want the Exec value
                                    exec_command: parsed_entry
                                        .section("Desktop Entry")
                                        .attr("Exec")
                                        .ok_or(DetectEditorError::ExecFieldMissing)?
                                        .into(),
                                    // The icon is optional, we try getting it because the Exec value may need it
                                    icon: parsed_entry
                                        .section("Desktop Entry")
                                        .attr("Icon")
                                        .map(Into::into),
                                    xdg_entry_path: entry_filepath,
                                })
                            })
                    })
            })
            // If this returns None, no errors ocurred, and no elements were found
            .unwrap_or(Err(DetectEditorError::FreeDesktopEntryNotFound))
    }

    pub fn open_file(&self, path: impl AsRef<Path>) -> Result<(), OpenFileError> {
        let path = path.as_ref();

        let maybe_icon = self.icon.as_ref().map(|icon_str| icon_str.as_os_str());

        // Parse the xdg command field with all the needed data
        let command_parts = xdg::parse_command(
            &self.exec_command,
            path.as_os_str(),
            maybe_icon,
            Some(&self.xdg_entry_path),
        );

        if !command_parts.is_empty() {
            // If command_parts has at least one element this works. If it has a single
            // element, &command_parts[1..] should be an empty slice (&[]) and bossy
            // `with_args` does not add any argument on that case, although the docs
            // do not make it obvious.
            bossy::Command::impure(&command_parts[0])
                .with_args(&command_parts[1..])
                .run_and_detach()
                .map_err(OpenFileError::LaunchFailed)
        } else {
            Err(OpenFileError::CommandParsingFailed)
        }
    }
}

pub fn open_file_with(
    application: impl AsRef<OsStr>,
    path: impl AsRef<OsStr>,
) -> bossy::Result<()> {
    let app_str = application.as_ref();
    let path_str = path.as_ref();

    let command_parts = xdg::get_xdg_data_dirs()
        .iter()
        .find_map(|dir| {
            let dir = dir.join("applications");
            let (entry, entry_path) = xdg::find_entry_by_app_name(&dir, &app_str)?;

            let command_parts = entry
                .section("Desktop Entry")
                .attr("Exec")
                .map(|str_entry| {
                    let osstring_entry: OsString = str_entry.into();
                    xdg::parse_command(
                        &osstring_entry,
                        path_str,
                        entry
                            .section("Desktop Entry")
                            .attr("Icon")
                            .map(|s| s.as_ref()),
                        Some(&entry_path),
                    )
                })?;
            // This could go outside, but we'd better have a proper error for it then
            if !command_parts.is_empty() {
                Some(command_parts) // This guarantees that command_parts has at least one element
            } else {
                None
            }
        })
        // Here is why we ought to change this function's return type, to fit this error
        .unwrap_or_else(|| vec![app_str.to_os_string()]);

    // If command_parts has at least one element, this won't panic from Out of Bounds
    bossy::Command::impure(&command_parts[0])
        .with_args(&command_parts[1..])
        .run_and_detach()
}

// We use "sh" in order to access "command -v", as that is a bultin command on sh.
// Linux does not require a binary "command" in path, so this seems the way to go.
#[cfg(target_os = "linux")]
pub fn command_path(name: &str) -> bossy::Result<bossy::Output> {
    bossy::Command::impure("sh")
        .with_args(&["-c", &format!("command -v {}", name)])
        .run_and_wait_for_output()
}
