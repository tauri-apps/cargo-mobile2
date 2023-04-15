use super::{config::Config, target::Target};
use crate::{
    os,
    target::TargetTrait as _,
    util::{
        cli::{Report, Reportable},
        ln, prefix_path,
    },
};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RemoveBrokenLinksError {
    #[error("Failed to list contents of jniLibs directory {dir}: {source}")]
    ReadDir {
        dir: PathBuf,
        source: std::io::Error,
    },
    #[error("Failed to get entry in jniLibs directory {dir}: {source}")]
    Entry {
        dir: PathBuf,
        source: std::io::Error,
    },
    #[error("Failed to remove broken symlink {path}: {source}")]
    Remove {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl Reportable for RemoveBrokenLinksError {
    fn report(&self) -> Report {
        match self {
            Self::ReadDir { dir, source } => Report::error(
                format!("Failed to list contents of jniLibs directory {:?}", dir),
                source,
            ),
            Self::Entry { dir, source } => Report::error(
                format!("Failed to get entry in jniLibs directory {:?}", dir),
                source,
            ),
            Self::Remove { path, source } => Report::error(
                format!("Failed to remove broken symlink {:?}", path),
                source,
            ),
        }
    }
}

#[derive(Debug, Error)]
pub enum SymlinkLibError {
    #[error("The symlink source is {0}, but nothing exists there")]
    SourceMissing(PathBuf),
    #[error(transparent)]
    SymlinkFailed(ln::Error),
}

impl Reportable for SymlinkLibError {
    fn report(&self) -> Report {
        Report::error("Failed to symlink lib", self)
    }
}

pub fn path(config: &Config, target: Target<'_>) -> PathBuf {
    prefix_path(
        config.project_dir(),
        format!("app/src/main/jniLibs/{}", &target.abi),
    )
}

#[derive(Debug)]
pub struct JniLibs {
    path: PathBuf,
}

impl JniLibs {
    pub fn create(config: &Config, target: Target<'_>) -> std::io::Result<Self> {
        let path = path(config, target);
        std::fs::create_dir_all(&path).map(|()| Self { path })
    }

    pub fn remove_broken_links(config: &Config) -> Result<(), RemoveBrokenLinksError> {
        for abi_dir in Target::all()
            .values()
            .map(|target| path(config, *target))
            .filter(|path| path.is_dir())
        {
            for entry in
                std::fs::read_dir(&abi_dir).map_err(|source| RemoveBrokenLinksError::ReadDir {
                    dir: abi_dir.clone(),
                    source,
                })?
            {
                let entry = entry
                    .map_err(|source| RemoveBrokenLinksError::Entry {
                        dir: abi_dir.clone(),
                        source,
                    })?
                    .path();
                if let Ok(path) = std::fs::read_link(&entry) {
                    log::info!("symlink at {:?} points to {:?}", entry, path);
                    if !path.exists() {
                        log::info!(
                            "deleting broken symlink {:?} (points to {:?}, which doesn't exist)",
                            entry,
                            path
                        );
                        std::fs::remove_file(entry)
                            .map_err(|source| RemoveBrokenLinksError::Remove { path, source })?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn symlink_lib(&self, src: &Path) -> Result<(), SymlinkLibError> {
        log::info!("symlinking lib {:?} in jniLibs dir {:?}", src, self.path);
        if src.is_file() {
            let dest = self.path.join(
                src.file_name()
                    .expect("developer error: file had no file name"),
            );
            os::ln::force_symlink(src, dest, ln::TargetStyle::File)
                .map_err(SymlinkLibError::SymlinkFailed)?;
            Ok(())
        } else {
            Err(SymlinkLibError::SourceMissing(src.to_owned()))
        }
    }
}
