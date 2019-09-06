use crate::{config::Config, opts::Clobbering, templating::template_pack, util};
use into_result::command::CommandError;
use regex::Regex;
use std::{
    ffi::OsStr,
    fmt, io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum Error {
    MissingTemplatePack {
        name: &'static str,
    },
    TemplateTraversalFailed(bicycle::TraversalError),
    TemplateProcessingFailed(bicycle::ProcessingError),
    GitInitFailed(CommandError),
    GitSubmoduleStatusFailed {
        name: String,
        cause: io::Error,
    },
    AppRootOutsideProject {
        app_root: PathBuf,
        project_root: PathBuf,
    },
    AppRootInvalidUtf8 {
        app_root: PathBuf,
    },
    GitSubmoduleAddFailed {
        name: String,
        cause: CommandError,
    },
    GitSubmoduleInitFailed {
        name: String,
        cause: CommandError,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::MissingTemplatePack { name } => {
                write!(f, "The {:?} template pack is missing.", name)
            }
            Error::TemplateTraversalFailed(err) => write!(f, "Template traversal failed: {}", err),
            Error::TemplateProcessingFailed(err) => {
                write!(f, "Template processing failed: {}", err)
            }
            Error::GitInitFailed(err) => write!(f, "Failed to initialize git: {}", err),
            Error::GitSubmoduleStatusFailed { name, cause } => write!(
                f,
                "Failed to check \".gitmodules\" for submodule {:?}: {}",
                name, cause,
            ),
            Error::AppRootOutsideProject { app_root, project_root } => write!(
                f,
                "The app root ({:?}) is outside of the project root ({:?}), which is pretty darn invalid.",
                app_root, project_root
            ),
            Error::AppRootInvalidUtf8 { app_root } => write!(f, "The app root ({:?}) contains invalid UTF-8.", app_root),
            Error::GitSubmoduleAddFailed { name, cause } => {
                write!(f, "Failed to add submodule {:?}: {}", name, cause)
            }
            Error::GitSubmoduleInitFailed { name, cause } => {
                write!(f, "Failed to init submodule {:?}: {}", name, cause)
            }
        }
    }
}

pub fn git_init(root: &Path) -> Result<(), Error> {
    if !root.join(".git").exists() {
        util::git(&root, &["init"]).map_err(Error::GitInitFailed)?;
    }
    Ok(())
}

pub fn submodule_exists(root: &Path, name: &str) -> io::Result<bool> {
    lazy_static::lazy_static! {
        static ref SUBMODULE_NAME_RE: Regex = Regex::new(r#"\[submodule "(.*)"\]"#).unwrap();
    }
    let path = root.join(".git/config");
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
) -> Result<(), Error> {
    let submodule_exists =
        submodule_exists(root, name).map_err(|cause| Error::GitSubmoduleStatusFailed {
            name: name.to_owned(),
            cause,
        })?;
    if !submodule_exists {
        let path = config
            .unprefix_path(config.app_root())
            .map_err(|_| Error::AppRootOutsideProject {
                app_root: config.app_root(),
                project_root: config.project_root().to_owned(),
            })?
            .join(path.as_ref());
        let path_str = path.to_str().ok_or_else(|| Error::AppRootInvalidUtf8 {
            app_root: config.app_root(),
        })?;
        util::git(
            &root,
            &["submodule", "add", "--name", name, remote, path_str],
        )
        .map_err(|cause| Error::GitSubmoduleAddFailed {
            name: name.to_owned(),
            cause,
        })?;
        util::git(&root, &["submodule", "update", "--init", "--recursive"]).map_err(|cause| {
            Error::GitSubmoduleInitFailed {
                name: name.to_owned(),
                cause,
            }
        })?;
    }
    Ok(())
}

pub fn hello_world(
    config: &Config,
    bike: &bicycle::Bicycle,
    clobbering: Clobbering,
) -> Result<(), Error> {
    let dest = config.project_root();
    git_init(&dest)?;
    submodule_init(
        config,
        &dest,
        "rust_lib",
        "git@bitbucket.org:brainium/rust_lib.git",
        "rust-lib",
    )?;

    let insert_data = |map: &mut bicycle::JsonMap| {
        config.insert_template_data(map);
        let app_root = config.app_root();
        map.insert("app_root", &app_root);
    };
    let actions = bicycle::traverse(
        template_pack(Some(config), "rust_lib_app").ok_or_else(|| Error::MissingTemplatePack {
            name: "rust_lib_app",
        })?,
        &dest,
        |path| bike.transform_path(path, insert_data),
    )
    .map_err(Error::TemplateTraversalFailed)?;
    // Prevent clobbering
    let actions = actions
        .iter()
        .filter(|action| clobbering.is_allowed() || !action.dest().exists());
    bike.process_actions(actions, insert_data)
        .map_err(Error::TemplateProcessingFailed)?;

    Ok(())
}
