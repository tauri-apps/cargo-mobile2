use crate::{
    config::Config,
    opts::Clobbering,
    templating::RemotePackResolveError,
    util::{
        cli::{Report, Reportable},
        Git,
    },
};
use std::path::PathBuf;

#[derive(Debug)]
pub enum Error {
    GitInitFailed(bossy::Error),
    TemplatePackResolveFailed(RemotePackResolveError),
    TraversalFailed {
        src: PathBuf,
        dest: PathBuf,
        cause: bicycle::TraversalError,
    },
    ProcessingFailed(bicycle::ProcessingError),
}

impl Reportable for Error {
    fn report(&self) -> Report {
        match self {
            Self::GitInitFailed(err) => Report::error("Failed to initialize git", err),
            Self::TemplatePackResolveFailed(err) => {
                Report::error("Failed to resolve template pack", err)
            }
            Self::TraversalFailed { src, dest, cause } => Report::error(
                format!(
                    "Base project template traversal from src {:?} to dest {:?} failed",
                    src, dest
                ),
                cause,
            ),
            Self::ProcessingFailed(err) => {
                Report::error("Base project template processing failed", err)
            }
        }
    }
}

pub fn gen(config: &Config, bike: &bicycle::Bicycle, clobbering: Clobbering) -> Result<(), Error> {
    let root = config.app().root_dir();

    let git = Git::new(&root);
    git.init().map_err(Error::GitInitFailed)?;
    let template_pack = config
        .app()
        .template_pack()
        .resolve(git)
        .map_err(Error::TemplatePackResolveFailed)?;
    log::info!("traversing template pack {:#?}", template_pack);
    let actions = bicycle::traverse(
        &template_pack,
        &root,
        |path| bike.transform_path(path, |_| ()),
        bicycle::DEFAULT_TEMPLATE_EXT,
    )
    .map_err(|cause| Error::TraversalFailed {
        src: template_pack.to_owned(),
        dest: root.to_owned(),
        cause,
    })?;
    bike.process_actions(
        actions
            .iter()
            // Prevent clobbering
            .filter(|action| {
                clobbering.allowed() || action.is_create_directory() || {
                    let dest = action.dest();
                    let parent = dest
                        .parent()
                        .expect("developer error: template dest wasn't in project root");
                    if parent == root {
                        !dest.exists()
                    } else {
                        // don't intrude upon existing directories
                        !parent.exists()
                    }
                }
            }),
        |_| (),
    )
    .map_err(Error::ProcessingFailed)?;

    Ok(())
}
