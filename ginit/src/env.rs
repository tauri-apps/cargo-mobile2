use crate::util::pure_command::ExplicitEnv;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    HomeNotSet(std::env::VarError),
    PathNotSet(std::env::VarError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::HomeNotSet(err) => write!(
                f,
                "The `HOME` environment variable isn't set, which is pretty weird: {}",
                err
            ),
            Error::PathNotSet(err) => write!(
                f,
                "The `PATH` environment variable isn't set, which is super weird: {}",
                err
            ),
        }
    }
}

#[derive(Debug)]
pub struct Env {
    home: String,
    path: String,
    term: Option<String>,
}

impl Env {
    pub fn new() -> Result<Self, Error> {
        let home = std::env::var("HOME").map_err(Error::HomeNotSet)?;
        let path = std::env::var("PATH").map_err(Error::PathNotSet)?;
        let term = std::env::var("TERM").ok();
        Ok(Self { home, path, term })
    }
}

// reminder to tomorrow fran: we need to impl display for Error

impl ExplicitEnv for Env {
    fn explicit_env(&self) -> Vec<(&str, &std::ffi::OsStr)> {
        let mut env = vec![("HOME", self.home.as_ref()), ("PATH", self.path.as_ref())];
        if let Some(term) = self.term.as_ref() {
            env.push(("TERM", term.as_ref()));
        }
        env
    }
}
