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
    GitInit(std::io::Error),
    TemplatePackResolve(FancyPackResolveError),
    Processing {
        src: PathBuf,
        dest: PathBuf,
        cause: bicycle::ProcessingError,
    },
}

impl Reportable for Error {
    fn report(&self) -> Report {
        match self {
            Self::GitInit(err) => Report::error("Failed to initialize git", err),
            Self::TemplatePackResolve(err) => Report::error("Failed to resolve template pack", err),
            Self::Processing { src, dest, cause } => Report::error(
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
    let git = Git::new(root);
    git.init().map_err(Error::GitInit)?;
    let pack_chain = config
        .app()
        .template_pack()
        .resolve(git, submodule_commit.as_deref())
        .map_err(Error::TemplatePackResolve)?;
    log::info!("template pack chain: {:#?}", pack_chain);
    for pack in pack_chain {
        log::info!("traversing template pack {:#?}", pack);
        bike.filter_and_process(pack, root, |_| (), filter.fun())
            .map_err(|cause| Error::Processing {
                src: pack.to_owned(),
                dest: root.to_owned(),
                cause,
            })?;
    }
    Ok(())
}
