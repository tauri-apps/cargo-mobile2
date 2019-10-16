mod cargo;
pub mod cli;
mod common_email_providers;
pub mod ln;
mod path;
pub mod prompt;
pub mod pure_command;
pub mod re;

pub use self::{cargo::CargoCommand, common_email_providers::COMMON_EMAIL_PROVIDERS, path::*};
use into_result::{
    command::{CommandError, CommandResult},
    IntoResult as _,
};
use std::{
    env,
    ffi::OsStr,
    fmt::{self, Display},
    fs::File,
    io::{self, Read, Write},
    path::Path,
    process::{Command, Stdio},
};

#[derive(Debug)]
pub enum Never {}

impl Display for Never {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unreachable!()
    }
}

#[derive(Debug)]
pub enum InitTextWrapperError {
    HyphenationLoadFailed(hyphenation::load::Error),
}

impl Display for InitTextWrapperError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InitTextWrapperError::HyphenationLoadFailed(err) => write!(
                f,
                "Failed to load hyphenation standard for \"en-US\": {}",
                err
            ),
        }
    }
}

pub type TextWrapper = textwrap::Wrapper<'static, hyphenation::Standard>;

pub fn init_text_wrapper() -> Result<TextWrapper, InitTextWrapperError> {
    use hyphenation::Load as _;
    let dictionary = hyphenation::Standard::from_embedded(hyphenation::Language::EnglishUS)
        .map_err(InitTextWrapperError::HyphenationLoadFailed)?;
    Ok(TextWrapper::with_splitter(
        textwrap::termwidth(),
        dictionary,
    ))
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

pub fn read_string(path: impl AsRef<OsStr>) -> io::Result<String> {
    File::open(path.as_ref()).and_then(|mut file| {
        let mut buf = String::new();
        file.read_to_string(&mut buf).map(|_| buf)
    })
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

#[cfg(target_os = "macos")]
pub fn open_in_editor(path: impl AsRef<OsStr>) -> CommandResult<()> {
    Command::new("open")
        .args(&["-a", "Visual Studio Code"])
        .arg(path.as_ref())
        .status()
        .into_result()
}

#[cfg(not(target_os = "macos"))]
pub fn open_in_editor(_path: impl AsRef<Path>) -> CommandResult<()> {
    unimplemented!()
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
