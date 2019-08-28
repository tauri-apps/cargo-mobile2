use crate::util::pure_command::ExplicitEnv;
use std::{env::VarError, ffi::OsStr};

#[derive(Debug)]
pub enum EnvError {
    HomeNotSet(VarError),
    PathNotSet(VarError),
    TermNotSet(VarError),
}

#[derive(Debug)]
pub struct Env {
    home: String,
    path: String,
    term: String,
}

impl Env {
    pub fn new() -> Result<Self, EnvError> {
        let home = std::env::var("HOME").map_err(EnvError::HomeNotSet)?;
        let path = std::env::var("PATH").map_err(EnvError::PathNotSet)?;
        let term = std::env::var("TERM").map_err(EnvError::TermNotSet)?;
        Ok(Self { home, path, term })
    }
}

impl ExplicitEnv for Env {
    fn explicit_env(&self) -> Vec<(&str, &OsStr)> {
        vec![
            ("HOME", self.home.as_ref()),
            ("PATH", self.path.as_ref()),
            ("TERM", self.term.as_ref()),
        ]
    }
}
