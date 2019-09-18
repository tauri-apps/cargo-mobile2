// This module will be axed prior to release

mod bin_target;
mod rust_dir;
mod rust_lib;
mod type_state;
mod user_code;

use self::{
    bin_target::BinTarget, rust_dir::RustDir, rust_lib::RustLib, type_state::*, user_code::UserCode,
};
use crate::config::Config;
use into_result::command::CommandError;
use std::{fmt, fs, io};

#[derive(Debug)]
pub enum Error {
    BinTargetMoveFailed(io::Error),
    GenDirCreationFailed(io::Error),
    RustDirDeletionFailed(io::Error),
    RustLibMoveFailed(CommandError),
    UserCodeMoveFailed(io::Error),
    UserCodeUpdateFailed(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BinTargetMoveFailed(err) => write!(f, "Failed to move bin target: {}", err),
            Error::GenDirCreationFailed(err) => {
                write!(f, "Failed to create \"gen\" directory: {}", err)
            }
            Error::RustDirDeletionFailed(err) => {
                write!(f, "Failed to delete \"rust\" directory: {}", err)
            }
            Error::RustLibMoveFailed(err) => {
                write!(f, "Failed to move \"rust/lib\" to \"rust-lib\": {}", err)
            }
            Error::UserCodeMoveFailed(err) => write!(f, "Failed to move user code: {}", err),
            Error::UserCodeUpdateFailed(err) => {
                write!(f, "Failed to update user \"Cargo.toml\": {}", err)
            }
        }
    }
}

#[derive(Debug)]
pub struct LegacyProject {
    bin_target: BinTarget<Legacy>,
    rust_dir: RustDir<Legacy>,
    rust_lib: RustLib<Legacy>,
    user_code: UserCode<Legacy>,
}

impl LegacyProject {
    // Determines if a project is (probably) using the old project structure
    pub fn heuristic_detect(config: &Config) -> Option<Self> {
        let rust_dir = config.app_root().join("rust");
        if rust_dir.is_dir()
            && rust_dir.join("lib").is_dir()
            && rust_dir.join(config.app_name()).is_dir()
            && rust_dir
                .join(format!("{}-desktop", config.app_name()))
                .is_dir()
        {
            Some(Self {
                bin_target: BinTarget::new(),
                rust_dir: RustDir::new(),
                rust_lib: RustLib::new(),
                user_code: UserCode::new(),
            })
        } else {
            None
        }
    }

    pub fn migrate(self, config: &Config) -> Result<MigratedProject, Error> {
        fs::create_dir_all(config.app_root().join("gen")).map_err(Error::GenDirCreationFailed)?;
        let bin_target = self
            .bin_target
            .move_to_gen(config)
            .map_err(Error::BinTargetMoveFailed)?;
        let rust_lib = self
            .rust_lib
            .move_to_root(config)
            .map_err(Error::RustLibMoveFailed)?;
        let user_code = self
            .user_code
            .move_to_root(config)
            .map_err(Error::UserCodeMoveFailed)?;
        let rust_dir = self
            .rust_dir
            .delete(config, &bin_target, &rust_lib, &user_code)
            .map_err(Error::RustDirDeletionFailed)?;
        let user_code = user_code
            .update_cargo_toml(config)
            .map_err(Error::UserCodeUpdateFailed)?;
        Ok(MigratedProject {
            bin_target,
            rust_dir,
            rust_lib,
            user_code,
        })
    }
}

#[derive(Debug)]
pub struct MigratedProject {
    bin_target: BinTarget<Moved>,
    rust_dir: RustDir<Deleted>,
    rust_lib: RustLib<Moved>,
    user_code: UserCode<Updated>,
}
