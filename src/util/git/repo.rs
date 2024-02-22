use crate::util::{self, Git};
use std::{
    ffi::OsStr,
    fmt::{self, Display},
    io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum Error {
    NoHomeDir(util::NoHomeDir),
    FetchFailed(std::io::Error),
    RevParseLocalFailed(std::io::Error),
    RevParseRemoteFailed(std::io::Error),
    LogFailed(std::io::Error),
    ParentDirCreationFailed { path: PathBuf, cause: io::Error },
    CloneFailed(std::io::Error),
    ResetFailed(std::io::Error),
    CleanFailed(std::io::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoHomeDir(err) => write!(f, "{}", err),
            Self::FetchFailed(err) => write!(f, "Failed to fetch repo: {}", err),
            Self::RevParseLocalFailed(err) => write!(f, "Failed to get checkout revision: {}", err),
            Self::RevParseRemoteFailed(err) => {
                write!(f, "Failed to get upstream revision: {}", err)
            }
            Self::LogFailed(err) => write!(f, "Failed to get commit log: {}", err),
            Self::ParentDirCreationFailed { path, cause } => {
                write!(f, "Failed to create parent directory {:?}: {}", path, cause)
            }
            Self::CloneFailed(err) => write!(f, "Failed to clone repo: {}", err),
            Self::ResetFailed(err) => write!(f, "Failed to reset repo: {}", err),
            Self::CleanFailed(err) => write!(f, "Failed to clean repo: {}", err),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Status {
    Stale,
    Fresh,
}

impl Status {
    pub fn stale(self) -> bool {
        matches!(self, Self::Stale)
    }

    pub fn fresh(self) -> bool {
        matches!(self, Self::Fresh)
    }
}

#[derive(Clone, Debug)]
pub struct Repo {
    path: PathBuf,
}

impl Repo {
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn checkouts_dir(checkout: impl AsRef<Path>) -> Result<Self, util::NoHomeDir> {
        util::checkouts_dir()
            .map(|dir| dir.join(checkout))
            .map(Self::from_path)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn git(&self) -> Git<'_> {
        Git::new(self.path())
    }

    pub fn status(&self) -> Result<Status, Error> {
        let status = if !self.path().is_dir() {
            Status::Stale
        } else {
            let git = self.git();
            git.command_parse("fetch origin")
                .run()
                .map_err(Error::FetchFailed)?;
            let local = git
                .command_parse("rev-parse HEAD")
                .stderr_capture()
                .stdout_capture()
                .run()
                .map_err(Error::RevParseLocalFailed)?;
            let remote = git
                .command_parse("rev-parse @{u}")
                .stderr_capture()
                .stdout_capture()
                .run()
                .map_err(Error::RevParseRemoteFailed)?;
            if local.stdout != remote.stdout {
                Status::Stale
            } else {
                Status::Fresh
            }
        };
        Ok(status)
    }

    pub fn latest_subject(&self) -> Result<String, Error> {
        self.git()
            .command_parse("log -1 --pretty=%s")
            .read()
            .map(|s| s.trim().to_owned())
            .map_err(Error::LogFailed)
    }

    pub fn latest_hash(&self) -> Result<String, Error> {
        self.git()
            .command_parse("log -1 --pretty=%H")
            .read()
            .map(|s| s.trim().to_owned())
            .map_err(Error::LogFailed)
    }

    pub fn update(&self, url: impl AsRef<OsStr>, branch: &str) -> Result<(), Error> {
        let path = self.path();
        if !path.is_dir() {
            let parent = self
                .path()
                .parent()
                .expect("developer error: `Repo` path was at root");
            if !parent.is_dir() {
                std::fs::create_dir_all(parent).map_err(|cause| {
                    Error::ParentDirCreationFailed {
                        path: parent.to_owned(),
                        cause,
                    }
                })?;
            }
            Git::new(parent)
                .command_parse(format!(
                    "clone --depth 1 --single-branch {} {}",
                    url.as_ref().to_string_lossy(),
                    path.to_string_lossy()
                ))
                .run()
                .map_err(Error::CloneFailed)?;
        } else {
            println!(
                "Updating `{}` repo...",
                Path::new(
                    self.path()
                        .file_name()
                        .expect("developer error: `Repo` path had no file name")
                )
                .display()
            );
            self.git()
                .command_parse("fetch --depth 1")
                .run()
                .map_err(Error::FetchFailed)?;
            self.git()
                .command_parse(format!("reset --hard origin/{branch}"))
                .run()
                .map_err(Error::ResetFailed)?;
            self.git()
                .command_parse("clean -dfx --exclude /target")
                .run()
                .map_err(Error::CleanFailed)?;
        }
        Ok(())
    }
}
