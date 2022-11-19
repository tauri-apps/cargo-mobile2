use std::{collections::HashMap, env, ffi::OsString, path::Path};

use crate::env::{Error, ExplicitEnv};

#[derive(Debug, Clone)]
pub struct Env {
    vars: HashMap<String, OsString>,
}

impl Env {
    pub fn new() -> Result<Self, Error> {
        let mut vars = HashMap::new();

        let path = env::var_os("Path").ok_or(Error::NotSet("PATH"))?;
        let pathext = env::var_os("PATHEXT").ok_or(Error::NotSet("PATHEXT"))?;
        let program_data = env::var_os("ProgramData").ok_or(Error::NotSet("ProgramData"))?;
        let system_root = env::var_os("SystemRoot").ok_or(Error::NotSet("SystemRoot"))?;
        let temp = env::var_os("TEMP").ok_or(Error::NotSet("TEMP"))?;
        let tmp = env::var_os("TMP").ok_or(Error::NotSet("TMP"))?;
        let userprofile = env::var_os("USERPROFILE").ok_or(Error::NotSet("USERPROFILE"))?;

        vars.insert("PATH".into(), path);
        vars.insert("PATHEXT".into(), pathext);
        vars.insert("ProgramData".into(), program_data);
        vars.insert("SystemRoot".into(), system_root);
        vars.insert("TEMP".into(), temp);
        vars.insert("TMP".into(), tmp);
        vars.insert("USERPROFILE".into(), userprofile);

        if let Some(ssh_auth_sock) = env::var_os("SSH_AUTH_SOCK") {
            vars.insert("SSH_AUTH_SOCK".into(), ssh_auth_sock);
        }
        if let Some(term) = env::var_os("TERM") {
            vars.insert("TERM".into(), term);
        }

        Ok(Self { vars })
    }

    pub fn path(&self) -> &OsString {
        self.vars.get("PATH").unwrap()
    }

    pub fn prepend_to_path(mut self, path: impl AsRef<Path>) -> Self {
        let mut path = path.as_ref().as_os_str().to_os_string();
        path.push(";");
        path.push(self.path().clone());
        self.vars.insert("PATH".into(), path);
        self
    }

    pub fn insert_env_var(&mut self, key: String, value: OsString) {
        self.vars.insert(key, value);
    }

    pub fn explicit_env_vars(mut self, vars: HashMap<String, OsString>) -> Self {
        self.vars.extend(vars);
        self
    }
}

impl ExplicitEnv for Env {
    fn explicit_env(&self) -> HashMap<String, OsString> {
        self.vars.clone()
    }
}
