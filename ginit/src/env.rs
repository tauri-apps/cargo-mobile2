use crate::util::pure_command::ExplicitEnv;
#[derive(Debug)]
pub enum EnvError {
    HomeNotSet(std::env::VarError),
    PathNotSet(std::env::VarError),
}

#[derive(Debug)]
pub struct Env {
    home: String,
    path: String,
    term: Option<String>,
    ssh_auth_sock: Option<String>,
}

impl Env {
    pub fn new() -> Result<Self, EnvError> {
        let home = std::env::var("HOME").map_err(EnvError::HomeNotSet)?;
        let path = std::env::var("PATH").map_err(EnvError::PathNotSet)?;
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
