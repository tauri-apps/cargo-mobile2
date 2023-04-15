//! Opinionated convenience wrapper for `std::process::Command` and friends.
//!
//! Note that this re-exports [`std::process::ChildStdin`],
//! [`std::process::ExitStatus`], and [`std::process::Stdio`], so the docs for
//! those items below might seem a bit out of place.

#![allow(unsafe_code)]

mod error;
mod handle;
mod output;

mod result {
    pub type Result<T> = std::result::Result<T, super::error::Error>;
}

pub use self::{error::*, handle::*, output::*, result::*};
pub use std::process::{ChildStderr, ChildStdin, ChildStdout, ExitStatus, Stdio};

use std::{
    ffi::OsStr,
    fmt::{self, Display},
    path::Path,
    process,
};

/// Build and run commands to your heart's content.
#[derive(Debug)]
pub struct Command {
    inner: process::Command,
    display: String,
}

impl Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}

impl Command {
    fn push_display(&mut self, component: &OsStr) {
        if !self.display.is_empty() {
            self.display.push(' ');
        }
        self.display.push_str(component.to_string_lossy().as_ref());
    }

    /// Start building a command that inherits all env vars from the environment.
    pub fn impure(name: impl AsRef<OsStr>) -> Self {
        let name = name.as_ref();
        let mut this = Self {
            inner: process::Command::new(name),
            display: Default::default(),
        };
        this.push_display(name);
        this
    }

    /// Start building a command with a completely clean environment. Note that
    /// at minimum, you'll often want to add `PATH` and `TERM` to the environment
    /// for things to function as expected.
    pub fn pure(name: impl AsRef<OsStr>) -> Self {
        let mut this = Self::impure(name);
        this.inner.env_clear();
        this
    }

    /// The same as `impure`, but parses the command from a string of
    /// whitespace-separated args, just like how you'd write the command in a
    /// terminal.
    pub fn try_impure_parse(arg_str: impl AsRef<str>) -> Option<Self> {
        let arg_str = arg_str.as_ref();
        let mut args = arg_str.split_whitespace();
        args.next().map(|name| {
            let mut this = Self::impure(name);
            this.add_args(args);
            this
        })
    }

    /// The same as `try_impure_parse`, but panics if given an empty string.
    pub fn impure_parse(arg_str: impl AsRef<str>) -> Self {
        Self::try_impure_parse(arg_str).expect("passed an empty string to `impure_parse`")
    }

    /// The same as `pure`, but parses the command from a string of
    /// whitespace-separated args, just like how you'd write the command in a
    /// terminal.
    pub fn try_pure_parse(arg_str: impl AsRef<str>) -> Option<Self> {
        let mut this = Self::try_impure_parse(arg_str);
        if let Some(this) = this.as_mut() {
            this.inner.env_clear();
        }
        this
    }

    /// The same as `try_pure_parse`, but panics if given an empty string.
    pub fn pure_parse(arg_str: impl AsRef<str>) -> Self {
        Self::try_pure_parse(arg_str).expect("passed an empty string to `pure_parse`")
    }

    /// Get the command's string representation.
    pub fn display(&self) -> &str {
        &self.display
    }

    pub fn set_stdin(&mut self, cfg: impl Into<Stdio>) -> &mut Self {
        let cfg = cfg.into();
        log::debug!("setting stdin to {:?} on command {:?}", cfg, self.display);
        self.inner.stdin(cfg);
        self
    }

    pub fn with_stdin(mut self, cfg: impl Into<Stdio>) -> Self {
        self.set_stdin(cfg);
        self
    }

    pub fn set_stdin_piped(&mut self) -> &mut Self {
        self.set_stdin(Stdio::piped());
        self
    }

    pub fn with_stdin_piped(mut self) -> Self {
        self.set_stdin_piped();
        self
    }

    pub fn set_stdin_null(&mut self) -> &mut Self {
        self.set_stdin(Stdio::null());
        self
    }

    pub fn with_stdin_null(mut self) -> Self {
        self.set_stdin_null();
        self
    }

    pub fn set_stdout(&mut self, cfg: impl Into<Stdio>) -> &mut Self {
        let cfg = cfg.into();
        log::debug!("setting stdout to {:?} on command {:?}", cfg, self.display);
        self.inner.stdout(cfg);
        self
    }

    pub fn with_stdout(mut self, cfg: impl Into<Stdio>) -> Self {
        self.set_stdout(cfg);
        self
    }

    pub fn set_stdout_piped(&mut self) -> &mut Self {
        self.set_stdout(Stdio::piped());
        self
    }

    pub fn with_stdout_piped(mut self) -> Self {
        self.set_stdout_piped();
        self
    }

    pub fn set_stdout_null(&mut self) -> &mut Self {
        self.set_stdout(Stdio::null());
        self
    }

    pub fn with_stdout_null(mut self) -> Self {
        self.set_stdout_null();
        self
    }

    pub fn set_current_dir<P: AsRef<Path>>(&mut self, dir: P) -> &mut Self {
        self.inner.current_dir(dir);
        self
    }

    pub fn with_current_dir<P: AsRef<Path>>(mut self, dir: P) -> Self {
        self.set_current_dir(dir);
        self
    }

    pub fn set_stderr(&mut self, cfg: impl Into<Stdio>) -> &mut Self {
        let cfg = cfg.into();
        log::debug!("setting stderr to {:?} on command {:?}", cfg, self.display);
        self.inner.stderr(cfg);
        self
    }

    pub fn with_stderr(mut self, cfg: impl Into<Stdio>) -> Self {
        self.set_stderr(cfg);
        self
    }

    pub fn set_stderr_piped(&mut self) -> &mut Self {
        self.set_stderr(Stdio::piped());
        self
    }

    pub fn with_stderr_piped(mut self) -> Self {
        self.set_stderr_piped();
        self
    }

    pub fn set_stderr_null(&mut self) -> &mut Self {
        self.set_stderr(Stdio::null());
        self
    }

    pub fn with_stderr_null(mut self) -> Self {
        self.set_stderr_null();
        self
    }

    pub fn add_env_var(&mut self, key: impl AsRef<OsStr>, val: impl AsRef<OsStr>) -> &mut Self {
        let key = key.as_ref();
        let val = val.as_ref();
        log::debug!(
            "adding env var {:?} = {:?} to command {:?}",
            key,
            val,
            self.display
        );
        self.inner.env(key, val);
        self
    }

    pub fn with_env_var(mut self, key: impl AsRef<OsStr>, val: impl AsRef<OsStr>) -> Self {
        self.add_env_var(key, val);
        self
    }

    pub fn add_env_vars(
        &mut self,
        vars: impl IntoIterator<Item = (impl AsRef<OsStr>, impl AsRef<OsStr>)>,
    ) -> &mut Self {
        for (key, val) in vars.into_iter() {
            self.add_env_var(key, val);
        }
        self
    }

    pub fn with_env_vars(
        mut self,
        vars: impl IntoIterator<Item = (impl AsRef<OsStr>, impl AsRef<OsStr>)>,
    ) -> Self {
        self.add_env_vars(vars);
        self
    }

    pub fn add_arg(&mut self, name: impl AsRef<OsStr>) -> &mut Self {
        let name = name.as_ref();
        log::debug!("adding arg {:?} to command {:?}", name, self.display);
        self.inner.arg(name);
        self.push_display(name);
        self
    }

    pub fn with_arg(mut self, name: impl AsRef<OsStr>) -> Self {
        self.add_arg(name);
        self
    }

    pub fn add_args(&mut self, args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> &mut Self {
        for arg in args.into_iter() {
            self.add_arg(arg);
        }
        self
    }

    pub fn with_args(mut self, args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> Self {
        self.add_args(args);
        self
    }

    /// The same as `add_args`, but parses the arg list from a string of
    /// whitespace-separated args, just like how you'd see them in a terminal.
    pub fn add_parsed_args(&mut self, arg_str: impl AsRef<str>) -> &mut Self {
        self.add_args(arg_str.as_ref().split_whitespace())
    }

    /// The same as `with_args`, but parses the arg list from a string of
    /// whitespace-separated args, just like how you'd see them in a terminal.
    pub fn with_parsed_args(mut self, arg_str: impl AsRef<str>) -> Self {
        self.add_parsed_args(arg_str);
        self
    }

    fn run_inner(&mut self) -> Result<Handle> {
        Error::from_child_result(self.display.clone(), self.inner.spawn())
    }

    /// Run the command and give you a delightful [`Handle`] to it. This allows
    /// you to decide when blocking should happen, but if you don't care, then
    /// [`Command::run_and_wait`] and [`Command::run_and_wait_for_output`] are
    /// better picks.
    pub fn run(&mut self) -> Result<Handle> {
        log::trace!("running command {:?}", self.display);
        self.set_stdout(os_pipe::dup_stdout().unwrap());
        self.set_stderr(os_pipe::dup_stderr().unwrap());
        self.run_inner()
    }

    /// Run the command and then detach it from the parent process. This allows
    /// the child process to outlive the parent process, akin to what can be
    /// achieved using `nohup` or `setsid`. This will automatically set stdin,
    /// stdout, and stderr to use [`Stdio::null`], so if you don't want that to
    /// happen, then you're screwed.
    pub fn run_and_detach(&mut self) -> Result<()> {
        log::trace!("running command {:?} and detaching", self.display);
        // This is pretty much lifted from the implementation in Alacritty:
        // https://github.com/alacritty/alacritty/blob/8bd2c13490f8cb6ad6b0c1104f9586b3554efea2/alacritty/src/daemon.rs
        #[cfg(unix)]
        unsafe {
            use std::os::unix::process::CommandExt as _;

            let display = self.display.clone();
            self.inner.pre_exec(move || match libc::fork() {
                -1 => {
                    let err = std::io::Error::last_os_error();
                    log::error!("`fork` failed for command {:?}: {}", display, err);
                    Err(err)
                }
                0 => {
                    if libc::setsid() == -1 {
                        let err = std::io::Error::last_os_error();
                        log::error!("`setsid` failed for command {:?}: {}", display, err);
                        Err(err)
                    } else {
                        Ok(())
                    }
                }
                _ => libc::_exit(0),
            });
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            use winapi::um::winbase::{CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW};

            self.inner
                .creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
        }
        self.set_stdin_null()
            .set_stdout_null()
            .set_stderr_null()
            .run_inner()
            .map(|handle| handle.leak())
    }

    /// Run the command and block until it exits.
    pub fn run_and_wait(&mut self) -> Result<ExitStatus> {
        log::trace!("running command {:?} and waiting for exit", self.display);
        self.set_stdout(os_pipe::dup_stdout().unwrap());
        self.set_stderr(os_pipe::dup_stderr().unwrap());
        self.inner
            .status()
            .map_err(|e| Error {
                command: self.display.clone(),
                cause: Box::new(Cause::SpawnFailed(e)),
            })
            .and_then(|status| Error::from_status_result(self.display.clone(), Ok(status)))
    }

    /// Run the command and block until its output is collected. This will
    /// automatically set stdout and stderr to use [`Stdio::piped`], so if you
    /// don't want that to happen, then you're screwed.
    pub fn run_and_wait_for_output(&mut self) -> Result<Output> {
        log::trace!("running command {:?} and waiting for output", self.display);
        Error::from_output_result(
            self.display.clone(),
            self.set_stdout_piped().set_stderr_piped().inner.output(),
        )
    }

    pub fn run_and_wait_for_str<T>(&mut self, f: impl FnOnce(&str) -> T) -> Result<T> {
        self.run_and_wait_for_output()?.stdout_str().map(f)
    }

    pub fn run_and_wait_for_string(&mut self) -> Result<String> {
        self.run_and_wait_for_str(ToOwned::to_owned)
    }
}
