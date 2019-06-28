use crate::{util, CONFIG};
use derive_more::From;
use lazy_static::lazy_static;
use regex::Regex;
use std::{ffi::OsStr, io, path::Path};

lazy_static! {
    static ref SUBMODULE_NAME_RE: Regex = Regex::new(r#"\[submodule "(.*)"\]"#).unwrap();
}

#[derive(Debug, From)]
pub enum ProjectCreationError {
    TemplateTraversalError(bicycle::TraversalError),
    TemplateProcessingError(bicycle::ProcessingError),
    GitInitError(util::CommandError),
    GitSubmoduleStatusError(io::Error),
    GitSubmoduleAddError(util::CommandError),
    GitSubmoduleInitError(util::CommandError),
}

fn git_init(root: &Path) -> Result<(), ProjectCreationError> {
    if !root.join(".git").exists() {
        util::git(&root, &["init"]).map_err(ProjectCreationError::GitInitError)?;
    }
    Ok(())
}

fn submodule_exists(root: &Path, name: &str) -> io::Result<bool> {
    let path = root.join(".gitmodules");
    if !path.exists() {
        Ok(false)
    } else {
        util::read_str(&path).map(|modules| util::has_match(&*SUBMODULE_NAME_RE, &modules, name))
    }
}

fn submodule_init(
    root: &Path,
    name: &str,
    remote: &str,
    path: impl AsRef<OsStr>,
) -> Result<(), ProjectCreationError> {
    let submodule_exists =
        submodule_exists(root, name).map_err(ProjectCreationError::GitSubmoduleStatusError)?;
    if !submodule_exists {
        let path = Path::new(&CONFIG.global.source_root).join(path.as_ref());
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

pub fn hello_world(bike: &bicycle::Bicycle, force: bool) -> Result<(), ProjectCreationError> {
    let template_dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates"));
    let dest = CONFIG.project_root();
    let insert_data = |map: &mut bicycle::JsonMap| {
        CONFIG.insert_data(map);
        let source_root = CONFIG.source_root();
        map.insert("source_root", &source_root);
    };
    let mut actions = bicycle::traverse(&template_dir.join("project_root"), &dest, |path| {
        bike.transform_path(path, insert_data)
    })?;
    actions.append(&mut bicycle::traverse(
        &template_dir.join("resources"),
        CONFIG.asset_path(),
        |path| bike.transform_path(path, insert_data),
    )?);
    // Prevent clobbering
    let actions = actions
        .iter()
        .filter(|action| force || !action.dest().exists());
    bike.process_actions(actions, insert_data)?;
    git_init(&dest)?;
    submodule_init(
        &dest,
        "rust_lib",
        "git@bitbucket.org:brainium/rust_lib.git",
        "lib",
    )?;
    Ok(())
}
