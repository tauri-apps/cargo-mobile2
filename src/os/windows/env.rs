use std::{env, path::Path};

use crate::env::{Error, ExplicitEnv};

#[derive(Debug, Clone)]
pub struct Env {
    path: String,
    pathext: String,
    program_data: String,
    system_root: String,
    temp: String,
    tmp: String,
    userprofile: String,
    ssh_auth_sock: Option<String>,
    term: Option<String>,
}

impl Env {
    pub fn new() -> Result<Self, Error> {
        let path = env::var("Path").map_err(Error::PathNotSet)?;
        let pathext = env::var("PATHEXT").map_err(Error::PathNotSet)?;
        let program_data =
            env::var("ProgramData").map_err(|err| Error::NotSet("ProgramData", err))?;
        let system_root = env::var("SystemRoot").map_err(|err| Error::NotSet("SystemRoot", err))?;
        let temp = env::var("TEMP").map_err(|err| Error::NotSet("TEMP", err))?;
        let tmp = env::var("TMP").map_err(|err| Error::NotSet("TMP", err))?;
        let userprofile =
            env::var("USERPROFILE").map_err(|err| Error::NotSet("USERPROFILE", err))?;
        let ssh_auth_sock = env::var("SSH_AUTH_SOCK").ok();
        let term = env::var("TERM").ok();
        Ok(Self {
            path,
            pathext,
            program_data,
            system_root,
            temp,
            tmp,
            userprofile,
            ssh_auth_sock,
            term,
        })
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn prepend_to_path(mut self, path: impl AsRef<Path>) -> Self {
        self.path = format!("{};{}", path.as_ref().display(), self.path);
        self
    }
}

impl ExplicitEnv for Env {
    fn explicit_env(&self) -> Vec<(&str, &std::ffi::OsStr)> {
        let mut env = vec![
            ("Path", self.path.as_ref()),
            ("PATHEXT", self.pathext.as_ref()),
            ("ProgramData", self.program_data.as_ref()),
            ("SystemRoot", self.system_root.as_ref()),
            ("TEMP", self.temp.as_ref()),
            ("TMP", self.tmp.as_ref()),
            ("USERPROFILE", self.userprofile.as_ref()),
        ];
        if let Some(ssh_auth_sock) = self.ssh_auth_sock.as_ref() {
            env.push(("SSH_AUTH_SOCK", ssh_auth_sock.as_ref()));
        }
        if let Some(term) = self.term.as_ref() {
            env.push(("TERM", term.as_ref()));
        }
        env
    }
}
