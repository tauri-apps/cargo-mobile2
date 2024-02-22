use crate::{
    util::{
        self,
        cli::{Report, TextWrapper},
        repo::{self, Repo},
    },
    DuctExpressionExt,
};
use std::{
    fmt::{self, Display},
    fs::{self, File},
    io,
    path::PathBuf,
};

static ENABLED_FEATURES: &[&str] = &[
    #[cfg(feature = "brainium")]
    "brainium",
    "cli",
];

#[derive(Debug)]
pub enum Error {
    NoHomeDir(util::NoHomeDir),
    StatusFailed(repo::Error),
    MarkerCreateFailed { path: PathBuf, cause: io::Error },
    UpdateFailed(repo::Error),
    InstallFailed(std::io::Error),
    MarkerDeleteFailed { path: PathBuf, cause: io::Error },
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoHomeDir(err) => write!(f, "{}", err),
            Self::StatusFailed(err) => {
                write!(f, "Failed to check status of `cargo-mobile2` repo: {}", err)
            }
            Self::MarkerCreateFailed { path, cause } => {
                write!(f, "Failed to create marker file at {:?}: {}", path, cause)
            }
            Self::UpdateFailed(err) => write!(f, "Failed to update `cargo-mobile2` repo: {}", err),
            Self::InstallFailed(err) => write!(
                f,
                "Failed to install new version of `cargo-mobile2`: {}",
                err
            ),
            Self::MarkerDeleteFailed { path, cause } => {
                write!(f, "Failed to delete marker file at {:?}: {}", path, cause)
            }
        }
    }
}

pub(crate) fn cargo_mobile_repo() -> Result<Repo, util::NoHomeDir> {
    Repo::checkouts_dir("cargo-mobile2")
}

pub(crate) fn updating_marker_path(repo: &Repo) -> PathBuf {
    repo.path()
        .parent()
        .expect("developer error: repo path had no parent")
        .parent()
        .expect("developer error: checkouts dir had no parent")
        .join(".updating")
}

pub fn update(wrapper: &TextWrapper) -> Result<(), Error> {
    let repo = cargo_mobile_repo().map_err(Error::NoHomeDir)?;
    let marker = updating_marker_path(&repo);
    let marker_exists = marker.is_file();
    if marker_exists {
        log::info!("marker file present at {:?}", marker);
    } else {
        log::info!("no marker file present at {:?}", marker);
    }
    let msg = if marker_exists || repo.status().map_err(Error::StatusFailed)?.stale() {
        File::create(&marker).map_err(|cause| Error::MarkerCreateFailed {
            path: marker.to_owned(),
            cause,
        })?;
        repo.update("https://github.com/tauri-apps/cargo-mobile2", "dev")
            .map_err(Error::UpdateFailed)?;
        println!("Installing updated `cargo-mobile2`...");
        let repo_c = repo.clone();
        duct::cmd("cargo", ["install", "--force", "--path"])
            .dup_stdio()
            .before_spawn(move |cmd| {
                cmd.arg(repo_c.path());
                cmd.args(["--no-default-features", "--features"]);
                cmd.arg(ENABLED_FEATURES.join(" "));
                Ok(())
            })
            .run()
            .map_err(Error::InstallFailed)?;
        fs::remove_file(&marker).map_err(|cause| Error::MarkerDeleteFailed {
            path: marker.to_owned(),
            cause,
        })?;
        log::info!("deleted marker file at {:?}", marker);
        "installed new version of `cargo-mobile2`"
    } else {
        "`cargo-mobile2` is already up-to-date"
    };
    let details = util::unwrap_either(
        repo.latest_subject()
            .map(util::format_commit_msg)
            .map_err(|err| format!("But we failed to get the latest commit message: {}", err)),
    );
    Report::victory(msg, details).print(wrapper);
    Ok(())
}
