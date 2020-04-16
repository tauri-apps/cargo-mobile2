use crate::util::cli::{Report, Reportable};
use std::{
    ffi::OsStr,
    fmt::{self, Debug, Display},
};

pub trait ExplicitEnv: Debug {
    fn explicit_env(&self) -> Vec<(&str, &OsStr)>;
}

#[derive(Debug)]
pub enum Error {
    HomeNotSet(std::env::VarError),
    PathNotSet(std::env::VarError),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HomeNotSet(err) => write!(
                f,
                "The `HOME` environment variable isn't set, which is pretty weird: {}",
                err
            ),
            Self::PathNotSet(err) => write!(
                f,
                "The `PATH` environment variable isn't set, which is super weird: {}",
                err
            ),
        }
    }
}

impl Reportable for Error {
    fn report(&self) -> Report {
        Report::error("Failed to initialize base environment", self)
    }
}

#[derive(Debug)]
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
