use super::{Error, ExitStatus, Result};
use std::{
    fmt::{self, Display},
    process, str,
};

#[derive(Clone, Copy, Debug)]
pub enum OutputStream {
    Out,
    Err,
}

impl Display for OutputStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl OutputStream {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Out => "stdout",
            Self::Err => "stderr",
        }
    }
}

/// All your output, in one convenient place! Wow!
#[derive(Debug)]
pub struct Output {
    command: String,
    inner: process::Output,
}

impl Output {
    pub(crate) fn new(command: String, inner: process::Output) -> Self {
        Self { command, inner }
    }

    pub fn status(&self) -> ExitStatus {
        self.inner.status
    }

    pub fn success(&self) -> bool {
        self.status().success()
    }

    pub(crate) fn stream(&self, stream: OutputStream) -> &[u8] {
        match stream {
            OutputStream::Out => &self.inner.stdout,
            OutputStream::Err => &self.inner.stderr,
        }
    }

    pub(crate) fn stream_str(&self, stream: OutputStream) -> Result<&str> {
        Error::from_utf8_result(&self.command, stream, str::from_utf8(self.stream(stream)))
    }

    pub fn stdout(&self) -> &[u8] {
        self.stream(OutputStream::Out)
    }

    pub fn stdout_str(&self) -> Result<&str> {
        self.stream_str(OutputStream::Out)
    }

    pub fn stderr(&self) -> &[u8] {
        self.stream(OutputStream::Err)
    }

    pub fn stderr_str(&self) -> Result<&str> {
        self.stream_str(OutputStream::Err)
    }
}
