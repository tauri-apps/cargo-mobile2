use crate::{
    config::Config,
    opts::Clobbering,
    util,
    util::{submodule, Git},
};
use std::{
    collections::VecDeque,
    fmt::{self, Display},
    path::PathBuf,
};

#[derive(Debug)]
pub enum Error {
    GitInitFailed(bossy::Error),
    SubmoduleFailed(submodule::Error),
    NoHomeDir(util::NoHomeDir),
    TraversalFailed {
        src: PathBuf,
        dest: PathBuf,
        cause: bicycle::TraversalError,
    },
    ProcessingFailed(bicycle::ProcessingError),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GitInitFailed(err) => write!(f, "Failed to initialize git: {}", err),
            Self::SubmoduleFailed(err) => write!(f, "{}", err),
            Self::NoHomeDir(err) => write!(f, "{}", err),
            Self::TraversalFailed { src, dest, cause } => write!(
                f,
                "Template traversal from src {:?} to dest {:?} failed: {}",
                src, dest, cause
            ),
            Self::ProcessingFailed(err) => write!(f, "Template processing failed: {}", err),
        }
    }
}

pub fn gen(config: &Config, bike: &bicycle::Bicycle, clobbering: Clobbering) -> Result<(), Error> {
    let root = config.app().root_dir();

    let git = Git::new(&root);
    git.init().map_err(Error::GitInitFailed)?;
    if let Some(submodules) = config.submodules() {
        for submodule in submodules {
            submodule.init(git).map_err(Error::SubmoduleFailed)?;
        }
    }

    if let Some(template_packs) = config.template_packs() {
        let mut actions = VecDeque::new();
        for template_pack in template_packs {
            log::info!("traversing template pack {:#?}", template_pack);
            let home = util::home_dir().map_err(Error::NoHomeDir)?;
            let src = template_pack.prefix_src(root, &home);
            let dest = template_pack.prefix_dest(root);
            actions.append(
                &mut bicycle::traverse(
                    &src,
                    &dest,
                    |path| bike.transform_path(path, |_| ()),
                    bicycle::DEFAULT_TEMPLATE_EXT,
                )
                .map_err(|cause| Error::TraversalFailed { src, dest, cause })?,
            );
        }
        bike.process_actions(
            actions
                .iter()
                // Prevent clobbering
                .filter(|action| clobbering.allowed() || !action.dest().exists()),
            |_| (),
        )
        .map_err(Error::ProcessingFailed)?;
    }

    Ok(())
}
