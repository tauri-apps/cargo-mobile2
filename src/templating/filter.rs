use crate::{
    bicycle::Action,
    config::{Config, Origin},
};
use ignore::gitignore::Gitignore;
use std::{
    fmt::{self, Display},
    io,
    path::PathBuf,
};

#[derive(Debug)]
pub enum FilterError {
    ReadDirFailed { path: PathBuf, cause: io::Error },
}

impl Display for FilterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadDirFailed { path, cause } => write!(
                f,
                "App root directory {:?} couldn't be checked for emptiness: {}",
                path, cause
            ),
        }
    }
}

#[derive(Debug)]
pub enum Filter {
    WildWest,
    Protected { unprotected: Gitignore },
}

impl Filter {
    pub fn new(
        config: &Config,
        config_origin: Origin,
        dot_first_init_exists: bool,
    ) -> Result<Self, FilterError> {
        if config_origin.freshly_minted() {
            log::info!("config freshly minted, so we're assuming a brand new project; using `WildWest` filtering strategy");
            Ok(Self::WildWest)
        } else if dot_first_init_exists {
            log::info!("`{}` exists, so we're assuming a brand new project; using `WildWest` filtering strategy", crate::init::DOT_FIRST_INIT_FILE_NAME);
            Ok(Self::WildWest)
        } else {
            log::info!("existing config loaded, so we're assuming an existing project; using `Protected` filtering strategy");
            let gitignore_path = config.app().root_dir().join(".gitignore");
            let (unprotected, err) = Gitignore::new(&gitignore_path);
            if let Some(err) = err {
                log::error!("non-fatal error loading {:?}: {}", gitignore_path, err);
            }
            if unprotected.is_empty() {
                log::warn!("no ignore entries were parsed from {:?}; project generation will more or less be a no-op", gitignore_path);
            } else {
                log::info!(
                    "{} ignore entries were parsed from {:?}",
                    unprotected.num_ignores(),
                    gitignore_path
                );
            }
            Ok(Self::Protected { unprotected })
        }
    }

    pub fn fun(&self) -> impl FnMut(&Action) -> bool + '_ {
        move |action| match self {
            Self::WildWest => {
                log::debug!(
                    "filtering strategy is `WildWest`, so action will be processed: {:#?}",
                    action
                );
                true
            }
            Self::Protected { unprotected } => {
                // If we're protecting the user's files, then we only allow
                // actions that apply to paths excluded from version control.
                let ignored = unprotected
                    .matched_path_or_any_parents(action.dest(), action.is_create_directory())
                    .is_ignore();
                if ignored {
                    log::debug!(
                        "action has unprotected src, so will be processed: {:#?}",
                        action
                    );
                } else {
                    log::debug!(
                        "action has protected src, so won't be processed: {:#?}",
                        action
                    );
                }
                ignored
            }
        }
    }
}
