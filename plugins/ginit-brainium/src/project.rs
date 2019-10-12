use crate::Brainium;
use ginit_core::{
    config::ConfigTrait as _,
    exports::{bicycle, into_result::command::CommandError},
    opts::Clobbering,
    template_pack, util,
};
use regex::Regex;
use std::{
    ffi::OsStr,
    fmt::{self, Display},
    io,
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
    GitSubmoduleAddFailed {
        name: String,
        cause: CommandError,
    },
    GitSubmoduleInitFailed {
        name: String,
        cause: CommandError,
    },
    RustLibTooOld,
    AppRootOutsideProject {
        app_root: PathBuf,
        project_root: PathBuf,
    },
    AppRootInvalidUtf8 {
        app_root: PathBuf,
    },
}

impl Display for Error {
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
            Error::GitSubmoduleAddFailed { name, cause } => {
                write!(f, "Failed to add submodule {:?}: {}", name, cause)
            }
            Error::GitSubmoduleInitFailed { name, cause } => {
                write!(f, "Failed to init submodule {:?}: {}", name, cause)
            }
            Error::RustLibTooOld => {
                write!(f, "The `rust-lib` you have checked out is too old to work with the new project structure. Please update it and then run this again.")
            }
            Error::AppRootOutsideProject { app_root, project_root } => write!(
                f,
                "The app root ({:?}) is outside of the project root ({:?}), which is pretty darn invalid.",
                app_root, project_root
            ),
            Error::AppRootInvalidUtf8 { app_root } => write!(f, "The app root ({:?}) contains invalid UTF-8.", app_root),
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
    config: &<Brainium as PluginTrait>::Config,
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
            .shared()
            .unprefix_path(config.shared().app_root())
            .map_err(|_| Error::AppRootOutsideProject {
                app_root: config.shared().app_root(),
                project_root: config.shared().project_root().to_owned(),
            })?
            .join(path.as_ref());
        let path_str = path.to_str().ok_or_else(|| Error::AppRootInvalidUtf8 {
            app_root: config.shared().app_root(),
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

pub fn generate(
    config: &<Brainium as PluginTrait>::Config,
    bike: &bicycle::Bicycle,
    clobbering: Clobbering,
) -> Result<(), Error> {
    let dest = config.shared().project_root();
    git_init(&dest)?;
    submodule_init(
        config,
        &dest,
        "rust_lib",
        "git@bitbucket.org:brainium/rust_lib.git",
        "rust-lib",
    )?;
    if !dest.join("rust-lib/templates/rust-lib-app").exists() {
        return Err(Error::RustLibTooOld);
    }

    let insert_data = |map: &mut bicycle::JsonMap| {
        config.insert_template_data(Brainium::NAME, map);
        let app_root = config.shared().app_root();
        map.insert("app-root", &app_root);
    };
    let actions = bicycle::traverse(
        template_pack!(Some(config.shared()), "rust-lib-app").ok_or_else(|| {
            Error::MissingTemplatePack {
                name: "rust-lib-app",
            }
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
