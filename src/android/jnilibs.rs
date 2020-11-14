use super::{config::Config, target::Target};
use crate::{
    target::TargetTrait as _,
    util::{
        cli::{Report, Reportable},
        ln,
    },
};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum SymlinkLibError {
    SourceMissing(PathBuf),
    SymlinkFailed(ln::Error),
}

impl Reportable for SymlinkLibError {
    fn report(&self) -> Report {
        let msg = "Failed to symlink lib";
        match self {
            Self::SourceMissing(src) => Report::error(
                msg,
                format!("The symlink source is {:?}, but nothing exists there", src),
            ),
            Self::SymlinkFailed(err) => Report::error(msg, err),
        }
    }
}

pub fn path(config: &Config, target: Target<'_>) -> PathBuf {
    config
        .project_dir()
        .join(format!("app/src/main/jniLibs/{}", &target.abi))
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

    pub fn remove_broken_links(config: &Config) -> std::io::Result<()> {
        for abi_dir in Target::all()
            .values()
            .map(|target| path(config, *target))
            .filter(|path| path.is_dir())
        {
            for entry in std::fs::read_dir(abi_dir)? {
                let entry = entry?.path();
                if let Ok(path) = std::fs::read_link(&entry) {
                    if !path.exists() {
                        log::info!(
                            "deleting broken symlink {:?} (points to {:?}, which doesn't exist)",
                            entry,
                            path
                        );
                        std::fs::remove_file(entry)?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn symlink_lib(&self, src: &Path) -> Result<(), SymlinkLibError> {
        log::info!("symlinking lib {:?} in jniLibs dir {:?}", src, self.path);
        if src.is_file() {
            ln::force_symlink(
                src,
                self.path.join(
                    src.file_name()
                        .expect("developer error: file had no file name"),
                ),
                ln::TargetStyle::File,
            )
            .map_err(SymlinkLibError::SymlinkFailed)
        } else {
            Err(SymlinkLibError::SourceMissing(src.to_owned()))
        }
    }
}
