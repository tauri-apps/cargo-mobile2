use super::{lfs, Git};
use once_cell_regex::regex;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum Cause {
    NameMissing,
    LfsFailed(lfs::Error),
    IndexCheckFailed(io::Error),
    InitCheckFailed(io::Error),
    PathInvalidUtf8,
    AddFailed(std::io::Error),
    InitFailed(std::io::Error),
    CheckoutFailed {
        commit: String,
        cause: std::io::Error,
    },
}

#[derive(Debug)]
pub struct Error {
    submodule: Submodule,
    cause: Box<Cause>,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &*self.cause {
            Cause::NameMissing => write!(
                f,
                "Failed to infer name for submodule at remote {:?}; please specify a name explicitly.",
                self.submodule.remote
            ),
            Cause::LfsFailed(err) => write!(
                f,
                "Failed to ensure presence of Git LFS for submodule {:?}: {}",
                self.submodule.name().unwrap(), err,
            ),
            Cause::IndexCheckFailed(err) => write!(
                f,
                "Failed to check \".gitmodules\" for submodule {:?}: {}",
                self.submodule.name().unwrap(), err,
            ),
            Cause::InitCheckFailed(err) => write!(
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
            Cause::CheckoutFailed { commit, cause } => write!(
                f,
                "Failed to checkout commit {:?} from submodule {:?} with remote {:?} and path {:?}: {}",
                commit, self.submodule.name().unwrap(), self.submodule.remote, self.submodule.path, cause
            ),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Submodule {
    name: Option<String>,
    remote: String,
    path: PathBuf,
    #[serde(default)]
    lfs: bool,
}

impl Submodule {
    pub fn with_remote_and_path(remote: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            name: None,
            remote: remote.into(),
            path: path.into(),
            lfs: false,
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

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn in_index(&self, git: Git<'_>, name: &str) -> io::Result<bool> {
        git.modules().map(|modules| {
            modules
                .filter(|modules| modules.contains(&format!("[submodule {:?}]", name)))
                .is_some()
        })
    }

    fn initialized(&self, git: Git<'_>, name: &str) -> io::Result<bool> {
        git.config().map(|config| {
            config
                .filter(|config| config.contains(&format!("[submodule {:?}]", name)))
                .is_some()
        })
    }

    pub fn init(&self, git: Git<'_>, commit: Option<&str>) -> Result<(), Error> {
        let name = self.name().ok_or_else(|| Error {
            submodule: self.clone(),
            cause: Box::new(Cause::NameMissing),
        })?;
        if self.lfs {
            lfs::ensure_present().map_err(|cause| Error {
                submodule: self.clone(),
                cause: Box::new(Cause::LfsFailed(cause)),
            })?;
        }
        let in_index = self.in_index(git, name).map_err(|cause| Error {
            submodule: self.clone(),
            cause: Box::new(Cause::IndexCheckFailed(cause)),
        })?;
        let initialized = if !in_index {
            let path_str = self
                .path
                .to_str()
                .ok_or_else(|| Error {
                    submodule: self.clone(),
                    cause: Box::new(Cause::PathInvalidUtf8),
                })?
                .to_owned();
            log::info!("adding submodule: {:#?}", self);
            let remote = self.remote.clone();
            let name = name.to_owned();
            git.command()
                .before_spawn(move |cmd| {
                    cmd.args(["submodule", "add", "--name", &name, &remote, &path_str]);
                    Ok(())
                })
                .run()
                .map_err(|cause| Error {
                    submodule: self.clone(),
                    cause: Box::new(Cause::AddFailed(cause)),
                })?;
            false
        } else {
            log::info!("submodule already in index: {:#?}", self);
            self.initialized(git, name).map_err(|cause| Error {
                submodule: self.clone(),
                cause: Box::new(Cause::InitCheckFailed(cause)),
            })?
        };
        if !initialized {
            log::info!("initializing submodule: {:#?}", self);
            git.command()
                .before_spawn(|cmd| {
                    cmd.args(["submodule", "update", "--init", "--recursive"]);
                    Ok(())
                })
                .run()
                .map_err(|cause| Error {
                    submodule: self.clone(),
                    cause: Box::new(Cause::InitFailed(cause)),
                })?;
        } else {
            log::info!("submodule already initalized: {:#?}", self);
        }
        if let Some(commit) = commit {
            let path = git.root().join(self.path());
            log::info!(
                "checking out commit {:?} in submodule at {:?}",
                commit,
                path
            );
            let commit = commit.to_owned();
            let commit_c = commit.clone();
            Git::new(&path)
                .command()
                .before_spawn(move |cmd| {
                    cmd.args(["checkout", &commit_c]);
                    Ok(())
                })
                .run()
                .map_err(|cause| Error {
                    submodule: self.clone(),
                    cause: Box::new(Cause::CheckoutFailed { commit, cause }),
                })?;
        }
        Ok(())
    }
}
