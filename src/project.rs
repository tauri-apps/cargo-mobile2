use crate::{
    bicycle,
    config::Config,
    templating::{self, FancyPackResolveError},
    util::{
        cli::{Report, Reportable},
        Git,
    },
};
use std::path::PathBuf;

#[derive(Debug)]
pub enum Error {
    GitInitFailed(bossy::Error),
    TemplatePackResolveFailed(FancyPackResolveError),
    ProcessingFailed {
        src: PathBuf,
        dest: PathBuf,
        cause: bicycle::ProcessingError,
    },
}

impl Reportable for Error {
    fn report(&self) -> Report {
        match self {
            Self::GitInitFailed(err) => Report::error("Failed to initialize git", err),
            Self::TemplatePackResolveFailed(err) => {
                Report::error("Failed to resolve template pack", err)
            }
            Self::ProcessingFailed { src, dest, cause } => Report::error(
                format!(
                    "Base project template processing from src {:?} to dest {:?} failed",
                    src, dest,
                ),
                cause,
            ),
        }
    }
}

pub fn gen(
    config: &Config,
    bike: &bicycle::Bicycle,
    filter: &templating::Filter,
    submodule_commit: Option<String>,
) -> Result<(), Error> {
    println!("Generating base project...");
    let root = config.app().root_dir();
    let git = Git::new(&root);
    git.init().map_err(Error::GitInitFailed)?;
    let pack_chain = config
        .app()
        .template_pack()
        .resolve(git, submodule_commit.as_deref())
        .map_err(Error::TemplatePackResolveFailed)?;
    log::info!("template pack chain: {:#?}", pack_chain);
    for pack in pack_chain {
        log::info!("traversing template pack {:#?}", pack);
        bike.filter_and_process(&pack, &root, |_| (), filter.fun())
            .map_err(|cause| Error::ProcessingFailed {
                src: pack.to_owned(),
                dest: root.to_owned(),
                cause,
            })?;
    }
    Ok(())
}
