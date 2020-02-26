mod cargo;
pub mod cli;
mod common_email_providers;
pub mod ln;
mod path;
pub mod prompt;
pub mod pure_command;
pub mod re;

pub use self::{cargo::CargoCommand, common_email_providers::COMMON_EMAIL_PROVIDERS, path::*};
use crate::{
    exports::into_result::{
        command::{CommandError, CommandResult},
        IntoResult as _,
    },
    os,
};
use std::{
    env,
    ffi::OsStr,
    fmt::{self, Display},
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub type Never = std::convert::Infallible;

pub type TextWrapper = textwrap::Wrapper<'static, textwrap::NoHyphenation>;

pub fn init_text_wrapper() -> TextWrapper {
    TextWrapper::with_splitter(textwrap::termwidth(), textwrap::NoHyphenation)
}

pub fn display(d: impl Display) -> String {
    format!("{}", d)
}

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

pub fn temp_dir() -> PathBuf {
    std::env::temp_dir().join("com.brainiumstudios.ginit")
}

pub fn add_to_path(path: impl Display) -> String {
    format!("{}:{}", path, env::var("PATH").unwrap())
}

pub fn command_path(name: &str) -> CommandResult<Vec<u8>> {
    Command::new("command")
        .arg("-v")
        .arg(name)
        .output()
        .into_result()
        .map(|output| output.stdout)
}

pub fn command_present(name: &str) -> CommandResult<bool> {
    command_path(name)
        .map(|_path| true)
        .or_else(|err| match err {
            CommandError::NonZeroExitStatus(Some(1)) => Ok(false),
            _ => Err(err),
        })
}

pub fn git(dir: &impl AsRef<Path>, args: &[impl AsRef<OsStr>]) -> CommandResult<()> {
    Command::new("git")
        .arg("-C")
        .arg(dir.as_ref())
        .args(args)
        .status()
        .into_result()
}

pub fn rustup_add(triple: &str) -> CommandResult<()> {
    Command::new("rustup")
        .args(&["target", "add", triple])
        .status()
        .into_result()
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

#[derive(Debug)]
pub enum PipeError {
    TxCommandFailed(CommandError),
    RxCommandFailed(CommandError),
    PipeFailed(io::Error),
}

impl Display for PipeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PipeError::TxCommandFailed(err) => write!(f, "Failed to run sending command: {}", err),
            PipeError::RxCommandFailed(err) => {
                write!(f, "Failed to run receiving command: {}", err)
            }
            PipeError::PipeFailed(err) => write!(f, "Failed to pipe output: {}", err),
        }
    }
}

pub fn pipe(mut tx_command: Command, mut rx_command: Command) -> Result<(), PipeError> {
    let tx_output = tx_command
        .output()
        .into_result()
        .map_err(PipeError::TxCommandFailed)?;
    let rx_command = rx_command
        .stdin(Stdio::piped())
        .spawn()
        .into_result()
        .map_err(PipeError::RxCommandFailed)?;
    rx_command
        .stdin
        .unwrap()
        .write_all(&tx_output.stdout)
        .map_err(PipeError::PipeFailed)
}
