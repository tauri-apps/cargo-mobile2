use crate::{config::Config, templating::template_pack, util};
use into_result::command::CommandError;
use regex::Regex;
use std::{ffi::OsStr, io, path::Path};

#[derive(Debug, derive_more::From)]
pub enum ProjectCreationError {
    MissingTemplatePack,
    TemplateTraversalError(bicycle::TraversalError),
    TemplateProcessingError(bicycle::ProcessingError),
    GitInitError(CommandError),
    GitSubmoduleStatusError(io::Error),
    GitSubmoduleAddError(CommandError),
    GitSubmoduleInitError(CommandError),
}

pub fn git_init(root: &Path) -> Result<(), ProjectCreationError> {
    if !root.join(".git").exists() {
        util::git(&root, &["init"]).map_err(ProjectCreationError::GitInitError)?;
    }
    Ok(())
}

pub fn submodule_exists(root: &Path, name: &str) -> io::Result<bool> {
    lazy_static::lazy_static! {
        static ref SUBMODULE_NAME_RE: Regex = Regex::new(r#"\[submodule "(.*)"\]"#).unwrap();
    }
    let path = root.join(".gitmodules");
    if !path.exists() {
        Ok(false)
    } else {
        util::read_str(&path).map(|modules| util::has_match(&*SUBMODULE_NAME_RE, &modules, name))
    }
}

pub fn submodule_init(
    config: &Config,
    root: &Path,
    name: &str,
    remote: &str,
    path: impl AsRef<OsStr>,
) -> Result<(), ProjectCreationError> {
    let submodule_exists =
        submodule_exists(root, name).map_err(ProjectCreationError::GitSubmoduleStatusError)?;
    if !submodule_exists {
        let path = config
            .unprefix_path(config.source_root())
            .expect("`source_root` outside of the project")
            .join(path.as_ref());
        let path_str = path
            .to_str()
            .expect("`source_root` contained invalid unicode");
        util::git(
            &root,
            &["submodule", "add", "--name", name, remote, path_str],
        )
        .map_err(ProjectCreationError::GitSubmoduleAddError)?;
        util::git(&root, &["submodule", "update", "--init", "--recursive"])
            .map_err(ProjectCreationError::GitSubmoduleInitError)?;
    }
    Ok(())
}

pub fn hello_world(
    config: &Config,
    bike: &bicycle::Bicycle,
    force: bool,
) -> Result<(), ProjectCreationError> {
    let dest = config.project_root();
    git_init(&dest)?;
    submodule_init(
        config,
        &dest,
        "rust_lib",
        "git@bitbucket.org:brainium/rust_lib.git",
        "lib",
    )?;

    let insert_data = |map: &mut bicycle::JsonMap| {
        config.insert_template_data(map);
        let source_root = config.source_root();
        map.insert("source_root", &source_root);
    };
    let mut actions = bicycle::traverse(
        template_pack(Some(config), "project_root")
            .ok_or_else(|| ProjectCreationError::MissingTemplatePack)?,
        &dest,
        |path| bike.transform_path(path, insert_data),
    )?;
    actions.append(&mut bicycle::traverse(
        template_pack(Some(config), "resources")
            .ok_or_else(|| ProjectCreationError::MissingTemplatePack)?,
        config.asset_path(),
        |path| bike.transform_path(path, insert_data),
    )?);
    // Prevent clobbering
    let actions = actions
        .iter()
        .filter(|action| force || !action.dest().exists());
    bike.process_actions(actions, insert_data)?;

    Ok(())
}
