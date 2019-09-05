mod cargo;
pub mod ln;
pub mod prompt;
pub mod pure_command;

pub use self::cargo::CargoCommand;
use into_result::{
    command::{CommandError, CommandResult},
    IntoResult as _,
};
use regex::Regex;
use std::{
    env,
    ffi::OsStr,
    fmt,
    fs::File,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub fn list_display(list: &[impl fmt::Display]) -> String {
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

pub fn read_str(path: impl AsRef<OsStr>) -> io::Result<String> {
    File::open(path.as_ref()).and_then(|mut file| {
        let mut buf = String::new();
        file.read_to_string(&mut buf).map(|_| buf)
    })
}

pub fn has_match(re: &Regex, body: &str, pattern: &str) -> bool {
    re.captures(body)
        .and_then(|caps| {
            caps.iter()
                .find(|cap| cap.map(|cap| cap.as_str() == pattern).unwrap_or_default())
        })
        .is_some()
}

// yay for bad string ergonomics
// https://github.com/rust-lang/rust/issues/42671
pub trait FriendlyContains<T>
where
    str: PartialEq<T>,
{
    fn friendly_contains(&self, value: &str) -> bool;
}

impl<T> FriendlyContains<T> for &[T]
where
    str: PartialEq<T>,
{
    fn friendly_contains(&self, value: &str) -> bool {
        self.iter().any(|item| value == item)
    }
}

pub fn add_to_path(path: impl fmt::Display) -> String {
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

fn common_root(abs_src: &Path, abs_dest: &Path) -> PathBuf {
    let mut dest_root = abs_dest.to_owned();
    loop {
        if abs_src.starts_with(&dest_root) {
            return dest_root;
        } else {
            if !dest_root.pop() {
                unreachable!("`abs_src` and `abs_dest` have no common root");
            }
        }
    }
}

pub fn relativize_path(abs_path: impl AsRef<Path>, abs_relative_to: impl AsRef<Path>) -> PathBuf {
    let (abs_path, abs_relative_to) = (abs_path.as_ref(), abs_relative_to.as_ref());
    assert!(abs_path.is_absolute());
    assert!(abs_relative_to.is_absolute());
    let (path, relative_to) = {
        let common_root = common_root(abs_path, abs_relative_to);
        let path = abs_path.strip_prefix(&common_root).unwrap();
        let relative_to = abs_relative_to.strip_prefix(&common_root).unwrap();
        (path, relative_to)
    };
    let mut rel_path = PathBuf::new();
    for _ in 0..relative_to.iter().count() {
        rel_path.push("..");
    }
    let rel_path = rel_path.join(path);
    log::info!(
        "{:?} relative to {:?} is {:?}",
        abs_path,
        abs_relative_to,
        rel_path
    );
    rel_path
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

#[derive(Debug, derive_more::From)]
pub enum PipeError {
    TxCommandError(CommandError),
    RxCommandError(CommandError),
    PipeError(io::Error),
}

pub fn pipe(mut tx_command: Command, mut rx_command: Command) -> Result<(), PipeError> {
    let tx_output = tx_command
        .output()
        .into_result()
        .map_err(PipeError::TxCommandError)?;
    let rx_command = rx_command
        .stdin(Stdio::piped())
        .spawn()
        .into_result()
        .map_err(PipeError::RxCommandError)?;
    rx_command
        .stdin
        .unwrap()
        .write_all(&tx_output.stdout)
        .map_err(From::from)
}
