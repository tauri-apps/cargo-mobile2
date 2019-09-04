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
}

impl Env {
    pub fn new() -> Result<Self, EnvError> {
        let home = std::env::var("HOME").map_err(EnvError::HomeNotSet)?;
        let path = std::env::var("PATH").map_err(EnvError::PathNotSet)?;
        let term = std::env::var("TERM").ok();
        Ok(Self { home, path, term })
    }
}

impl ExplicitEnv for Env {
    fn explicit_env(&self) -> Vec<(&str, &std::ffi::OsStr)> {
        let mut env = vec![("HOME", self.home.as_ref()), ("PATH", self.path.as_ref())];
        if let Some(term) = self.term.as_ref() {
            env.push(("TERM", term.as_ref()));
        }
        env
    }
}
