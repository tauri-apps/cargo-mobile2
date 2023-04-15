use super::{ExitStatus, Handle, Output, OutputStream, Result};
use std::{
    error::Error as StdError,
    fmt::{self, Display},
    io, process, str,
};

/// The specific cause of an [`Error`].
#[derive(Debug)]
pub enum Cause {
    SpawnFailed(io::Error),
    WaitFailed(io::Error),
    CommandFailed(ExitStatus),
    CommandFailedWithOutput(Output),
    InvalidUtf8 {
        stream: OutputStream,
        source: std::str::Utf8Error,
    },
}

impl Cause {
    pub(crate) fn from_io_err(err: io::Error) -> Self {
        Self::WaitFailed(err)
    }

    fn from_status(status: process::ExitStatus) -> std::result::Result<ExitStatus, Self> {
        if status.success() {
            Ok(status)
        } else {
            Err(Self::CommandFailed(status))
        }
    }

    fn from_output(command: String, output: process::Output) -> std::result::Result<Output, Self> {
        let output = Output::new(command, output);
        if output.success() {
            Ok(output)
        } else {
            Err(Self::CommandFailedWithOutput(output))
        }
    }

    fn status(&self) -> Option<ExitStatus> {
        if let Self::CommandFailed(status) = self {
            Some(*status)
        } else {
            self.output().map(|output| output.status())
        }
    }

    fn output(&self) -> Option<&Output> {
        if let Self::CommandFailedWithOutput(output) = self {
            Some(output)
        } else {
            None
        }
    }
}

/// The bearer of bad news.
#[derive(Debug)]
pub struct Error {
    pub(crate) command: String,
    pub(crate) cause: Box<Cause>,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn command_failed(
            f: &mut fmt::Formatter,
            command: &str,
            status: ExitStatus,
        ) -> fmt::Result {
            write!(f, "Command {:?} didn't complete successfully, ", command)?;
            if let Some(exit_code) = status.code() {
                write!(f, "exiting with code {}.", exit_code)
            } else {
                write!(f, "but returned no exit code.")
            }
        }

        match &*self.cause {
            Cause::SpawnFailed(err) => write!(
                f,
                "Failed to spawn child process for command {:?}: {}",
                self.command, err
            ),
            Cause::WaitFailed(err) => write!(
                f,
                "Failed to wait for child process for command {:?} to exit: {}",
                self.command, err
            ),
            Cause::CommandFailed(status) => command_failed(f, &self.command, *status),
            Cause::CommandFailedWithOutput(output) => {
                command_failed(f, &self.command, output.status())?;
                if !output.stderr().is_empty() {
                    write!(
                        f,
                        " stderr contents: {}",
                        String::from_utf8_lossy(output.stderr())
                    )
                } else {
                    write!(f, " stderr was empty.")
                }
            }
            Cause::InvalidUtf8 { stream, source, .. } => write!(
                f,
                "{} for command {:?} contained invalid UTF-8: {}",
                stream, self.command, source,
            ),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match &*self.cause {
            Cause::SpawnFailed(err) => Some(err as _),
            Cause::WaitFailed(err) => Some(err as _),
            Cause::InvalidUtf8 { source, .. } => Some(source as _),
            _ => None,
        }
    }
}

impl Error {
    pub(crate) fn from_status_result(
        command: String,
        result: io::Result<process::ExitStatus>,
    ) -> std::result::Result<ExitStatus, Self> {
        result
            .map_err(Cause::from_io_err)
            .and_then(Cause::from_status)
            .map_err(|cause| Self {
                command,
                cause: Box::new(cause),
            })
    }

    pub(crate) fn from_output_result(
        command: String,
        result: io::Result<process::Output>,
    ) -> std::result::Result<Output, Self> {
        result
            .map_err(Cause::from_io_err)
            .and_then(|output| Cause::from_output(command.clone(), output))
            .map_err(|cause| Self {
                command,
                cause: Box::new(cause),
            })
    }

    pub(crate) fn from_child_result(
        command: String,
        result: io::Result<process::Child>,
    ) -> std::result::Result<Handle, Self> {
        // `match` is favored here to avoid cloning `command`
        match result {
            Ok(child) => Ok(Handle::new(command, child)),
            Err(err) => Err(Self {
                command,
                cause: Box::new(Cause::from_io_err(err)),
            }),
        }
    }

    pub(crate) fn from_utf8_result<'a>(
        command: &str,
        stream: OutputStream,
        result: std::result::Result<&'a str, std::str::Utf8Error>,
    ) -> std::result::Result<&'a str, Self> {
        result.map_err(|source| Self {
            command: command.to_owned(),
            cause: Box::new(Cause::InvalidUtf8 { stream, source }),
        })
    }

    pub fn command(&self) -> &str {
        &self.command
    }

    pub fn status(&self) -> Option<ExitStatus> {
        self.cause.status()
    }

    pub fn code(&self) -> Option<i32> {
        self.status().and_then(|status| status.code())
    }

    pub fn output(&self) -> Option<&Output> {
        self.cause.output()
    }

    pub fn stdout(&self) -> Option<&[u8]> {
        self.output().map(|output| output.stdout())
    }

    pub fn stdout_str(&self) -> Option<Result<&str>> {
        self.output().map(|output| output.stdout_str())
    }

    pub fn stderr(&self) -> Option<&[u8]> {
        self.output().map(|output| output.stderr())
    }

    pub fn stderr_str(&self) -> Option<Result<&str>> {
        self.output().map(|output| output.stderr_str())
    }
}
