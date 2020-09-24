mod cargo;
pub mod cli;
mod git;
pub mod ln;
mod path;
pub mod prompt;

pub use self::{cargo::*, git::*, path::*};

use self::cli::{Report, Reportable};
use crate::os;
use once_cell_regex::regex;
use std::{
    fmt::{self, Display},
    io::{self, Write},
    path::Path,
};
use thiserror::Error;

pub fn list_display(list: &[impl Display]) -> String {
    if list.len() == 1 {
        list[0].to_string()
    } else if list.len() == 2 {
        format!("{} and {}", list[0], list[1])
    } else {
        let mut display = String::new();
        for (idx, item) in list.iter().enumerate() {
            let formatted = if idx + 1 == list.len() {
                // this is the last item
                format!("and {}", item)
            } else {
                format!("{}, ", item)
            };
            display.push_str(&formatted);
        }
        display
    }
}

pub fn reverse_domain(domain: &str) -> String {
    domain.split('.').rev().collect::<Vec<_>>().join(".")
}

pub fn rustup_add(triple: &str) -> bossy::Result<bossy::ExitStatus> {
    bossy::Command::impure("rustup")
        .with_args(&["target", "add", triple])
        .run_and_wait()
}

#[derive(Debug)]
pub enum HostTargetTripleError {
    CommandFailed(bossy::Error),
    Utf8Invalid(std::str::Utf8Error),
    NoMatchesFound(String),
}

impl Reportable for HostTargetTripleError {
    fn report(&self) -> Report {
        let msg = "Failed to detect host target triple";
        match self {
            Self::CommandFailed(err) => Report::error(msg, err),
            Self::Utf8Invalid(err) => Report::error(msg, err),
            Self::NoMatchesFound(output) => {
                Report::error(msg, format!("No matches found in output {:?}", output))
            }
        }
    }
}

pub fn host_target_triple() -> Result<String, HostTargetTripleError> {
    // TODO: add fast paths
    let output = bossy::Command::impure("rustc")
        .with_args(&["--verbose", "--version"])
        .run_and_wait_for_output()
        .map_err(HostTargetTripleError::CommandFailed)?;
    let raw = output
        .stdout_str()
        .map_err(HostTargetTripleError::Utf8Invalid)?;
    regex!(r"host: ([\w-]+)")
        .captures(raw)
        .map(|caps| {
            let triple = caps[1].to_owned();
            log::info!("detected host target triple {:?}", triple);
            triple
        })
        .ok_or_else(|| HostTargetTripleError::NoMatchesFound(raw.to_owned()))
}

#[derive(Debug, Error)]
pub enum RustVersionError {
    #[error("Failed to check rustc version: {0}")]
    CommandFailed(#[from] bossy::Error),
    #[error("Failed to parse rustc version info: {0:?}")]
    InvalidUtf8(#[from] std::str::Utf8Error),
    #[error("Failed to parse rustc version info: {0:?}")]
    ParseFailed(String),
    #[error("Failed to parse rustc major version from {version:?}: {source}")]
    MajorInvalid {
        version: String,
        source: std::num::ParseIntError,
    },
    #[error("Failed to parse rustc minor version from {version:?}: {source}")]
    MinorInvalid {
        version: String,
        source: std::num::ParseIntError,
    },
    #[error("Failed to parse rustc patch version from {version:?}: {source}")]
    PatchInvalid {
        version: String,
        source: std::num::ParseIntError,
    },
}

impl Reportable for RustVersionError {
    fn report(&self) -> Report {
        Report::error("Failed to check Rust version", self)
    }
}

#[derive(Debug)]
pub struct RustVersion {
    pub triple: (u32, u32, u32),
    pub flavor: Option<String>,
}

impl Display for RustVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.triple.0, self.triple.1, self.triple.2)?;
        if let Some(flavor) = &self.flavor {
            write!(f, "-{}", flavor)?;
        }
        Ok(())
    }
}

impl RustVersion {
    pub fn check() -> Result<Self, RustVersionError> {
        let output = bossy::Command::impure_parse("rustc --version").run_and_wait_for_output()?;
        let output = output.stdout_str()?;
        let re = regex!(
            r"rustc (?P<version>(?P<major>\d+)\.(?P<minor>\d+)\.(?P<patch>\d+)(-(?P<flavor>\w+))?)"
        );
        let caps = re
            .captures(output)
            .ok_or_else(|| RustVersionError::ParseFailed(output.to_owned()))?;
        let version_str = &caps["version"];
        let this = Self {
            triple: (
                caps["major"]
                    .parse::<u32>()
                    .map_err(|source| RustVersionError::MajorInvalid {
                        version: version_str.to_owned(),
                        source,
                    })?,
                caps["minor"]
                    .parse::<u32>()
                    .map_err(|source| RustVersionError::MinorInvalid {
                        version: version_str.to_owned(),
                        source,
                    })?,
                caps["patch"]
                    .parse::<u32>()
                    .map_err(|source| RustVersionError::PatchInvalid {
                        version: version_str.to_owned(),
                        source,
                    })?,
            ),
            flavor: caps.name("flavor").map(|flavor| flavor.as_str().to_owned()),
        };
        log::info!("detected rustc version {}", this);
        Ok(this)
    }
}

pub fn prepend_to_path(path: impl Display, base_path: impl Display) -> String {
    format!("{}:{}", path, base_path)
}

pub fn command_path(name: &str) -> bossy::Result<bossy::Output> {
    bossy::Command::impure("command")
        .with_args(&["-v", name])
        .run_and_wait_for_output()
}

pub fn command_present(name: &str) -> bossy::Result<bool> {
    command_path(name).map(|_path| true).or_else(|err| {
        if let Some(1) = err.status().and_then(|status| status.code()) {
            Ok(false)
        } else {
            Err(err)
        }
    })
}

#[derive(Debug)]
pub enum PipeError {
    TxCommandFailed(bossy::Error),
    RxCommandFailed(bossy::Error),
    PipeFailed(io::Error),
    WaitFailed(bossy::Error),
}

impl Display for PipeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TxCommandFailed(err) => write!(f, "Failed to run sending command: {}", err),
            Self::RxCommandFailed(err) => write!(f, "Failed to run receiving command: {}", err),
            Self::PipeFailed(err) => write!(f, "Failed to pipe output: {}", err),
            Self::WaitFailed(err) => {
                write!(f, "Failed to wait for receiving command to exit: {}", err)
            }
        }
    }
}

pub fn pipe(mut tx_command: bossy::Command, rx_command: bossy::Command) -> Result<bool, PipeError> {
    let tx_output = tx_command
        .run_and_wait_for_output()
        .map_err(PipeError::TxCommandFailed)?;
    if !tx_output.stdout().is_empty() {
        let mut rx_command = rx_command
            .with_stdin_piped()
            .with_stdout(bossy::Stdio::inherit())
            .run()
            .map_err(PipeError::RxCommandFailed)?;
        let pipe_result = rx_command
            .stdin()
            .expect("developer error: `rx_command` stdin not captured")
            .write_all(tx_output.stdout())
            .map_err(PipeError::PipeFailed);
        let wait_result = rx_command.wait_for_output().map_err(PipeError::WaitFailed);
        // We try to wait even if the pipe failed, but the pipe error has higher
        // priority than the wait error, since it's likely to be more relevant.
        pipe_result?;
        wait_result?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[derive(Debug)]
pub enum OpenInEditorError {
    DetectFailed(os::DetectEditorError),
    OpenFailed(os::OpenFileError),
}

impl Display for OpenInEditorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DetectFailed(err) => write!(f, "Failed to detect editor: {}", err),
            Self::OpenFailed(err) => write!(f, "Failed to open path in edtior: {}", err),
        }
    }
}

pub fn open_in_editor(path: impl AsRef<Path>) -> Result<(), OpenInEditorError> {
    let path = path.as_ref();
    os::Application::detect_editor()
        .map_err(OpenInEditorError::DetectFailed)?
        .open_file(path)
        .map_err(OpenInEditorError::OpenFailed)
}
