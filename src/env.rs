use crate::util::cli::{Report, Reportable};
use std::{ffi::OsStr, fmt::Debug, path::Path};
use thiserror::Error;

pub trait ExplicitEnv: Debug {
    fn explicit_env(&self) -> Vec<(&str, &OsStr)>;
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("The `HOME` environment variable isn't set, which is pretty weird: {0}")]
    HomeNotSet(#[source] std::env::VarError),
    #[error("The `PATH` environment variable isn't set, which is super weird: {0}")]
    PathNotSet(#[source] std::env::VarError),
}

impl Reportable for Error {
    fn report(&self) -> Report {
        Report::error("Failed to initialize base environment", self)
    }
}

#[derive(Clone, Debug)]
pub struct Env {
    home: String,
    path: String,
    term: Option<String>,
    ssh_auth_sock: Option<String>,
}

impl Env {
    pub fn new() -> Result<Self, Error> {
        let home = std::env::var("HOME").map_err(Error::HomeNotSet)?;
        let path = std::env::var("PATH").map_err(Error::PathNotSet)?;
        let term = std::env::var("TERM").ok();
        let ssh_auth_sock = std::env::var("SSH_AUTH_SOCK").ok();
        Ok(Self {
            home,
            path,
            term,
            ssh_auth_sock,
        })
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn prepend_to_path(mut self, path: impl AsRef<Path>) -> Self {
        self.path = format!("{}:{}", path.as_ref().display(), self.path);
        self
    }
}

impl ExplicitEnv for Env {
    fn explicit_env(&self) -> Vec<(&str, &std::ffi::OsStr)> {
        let mut env = vec![("HOME", self.home.as_ref()), ("PATH", self.path.as_ref())];
        if let Some(term) = self.term.as_ref() {
            env.push(("TERM", term.as_ref()));
        }
        if let Some(ssh_auth_sock) = self.ssh_auth_sock.as_ref() {
            env.push(("SSH_AUTH_SOCK", ssh_auth_sock.as_ref()));
        }
        env
    }
}
