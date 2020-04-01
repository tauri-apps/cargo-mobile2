use super::Git;
use once_cell_regex::regex;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    io,
    path::PathBuf,
};

#[derive(Debug)]
pub enum Cause {
    NameMissing,
    StatusFailed(io::Error),
    PathInvalidUtf8,
    AddFailed(bossy::Error),
    InitFailed(bossy::Error),
}

#[derive(Debug)]
pub struct Error {
    submodule: Submodule,
    cause: Cause,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.cause {
            Cause::NameMissing => write!(
                f,
                "Failed to infer name for submodule at remote {:?}; please specify a name explicitly.",
                self.submodule.remote
            ),
            Cause::StatusFailed(err) => write!(
                f,
                "Failed to check \".git/config\" for submodule {:?}: {}",
                self.submodule.name().unwrap(), err,
            ),
            Cause::PathInvalidUtf8 => write!(
                f,
                "Submodule path {:?} wasn't valid utf-8.",
                self.submodule.path,
            ),
            Cause::AddFailed(err) => write!(
                f,
                "Failed to add submodule {:?} with remote {:?} and path {:?}: {}",
                self.submodule.name().unwrap(), self.submodule.remote, self.submodule.path, err
            ),
            Cause::InitFailed(err) => write!(
                f,
                "Failed to init submodule {:?} with remote {:?} and path {:?}: {}",
                self.submodule.name().unwrap(), self.submodule.remote, self.submodule.path, err
            ),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Submodule {
    name: Option<String>,
    remote: String,
    path: PathBuf,
}

impl Submodule {
    pub fn with_remote_and_path(remote: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            name: None,
            remote: remote.into(),
            path: path.into(),
        }
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref().or_else(|| {
            let name = regex!(r"(?P<name>\w+)\.git")
                .captures(&self.remote)
                // Indexing would return `str` instead of `&str`, which doesn't
                // play nice with our lifetime needs here...
                .map(|caps| caps.name("name").unwrap().as_str());
            log::info!("detected submodule name: {:?}", name);
            name
        })
    }

    fn exists(&self, git: Git<'_>, name: &str) -> io::Result<bool> {
        git.config().map(|config| {
            config
                .filter(|config| config.contains(&format!("[submodule {:?}]", name)))
                .is_some()
        })
    }

    pub fn init(&self, git: Git<'_>) -> Result<(), Error> {
        let name = self.name().ok_or_else(|| Error {
            submodule: self.clone(),
            cause: Cause::NameMissing,
        })?;
        let exists = self.exists(git, &name).map_err(|cause| Error {
            submodule: self.clone(),
            cause: Cause::StatusFailed(cause),
        })?;
        if !exists {
            log::info!("initializing submodule: {:#?}", self);
            let path_str = self.path.to_str().ok_or_else(|| Error {
                submodule: self.clone(),
                cause: Cause::PathInvalidUtf8,
            })?;
            git.command()
                .with_args(&["submodule", "add", "--name", &name, &self.remote, path_str])
                .run_and_wait()
                .map_err(|cause| Error {
                    submodule: self.clone(),
                    cause: Cause::AddFailed(cause),
                })?;
            git.command()
                .with_args(&["submodule", "update", "--init", "--recursive"])
                .run_and_wait()
                .map_err(|cause| Error {
                    submodule: self.clone(),
                    cause: Cause::InitFailed(cause),
                })?;
        } else {
            log::info!("submodule already exists: {:#?}", self);
        }
        Ok(())
    }
}
