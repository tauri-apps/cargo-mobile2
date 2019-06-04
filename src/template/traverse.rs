use derive_more::From;
use std::{collections::VecDeque, fmt, fs, io, path::{Path, PathBuf}};

static TEMPLATE_EXT: &'static str = "hbs";

#[derive(Debug)]
pub enum Action {
    CreateDirectory {
        dest: PathBuf,
    },
    CopyFile {
        src: PathBuf,
        dest: PathBuf,
    },
    WriteTemplate {
        src: PathBuf,
        dest: PathBuf,
    },
}

impl Action {
    pub fn new_create_directory<E>(
        dest: &Path,
        transform_path: impl Fn(&Path) -> Result<PathBuf, E>,
    ) -> Result<Self, E> {
        Ok(Action::CreateDirectory {
            dest: transform_path(dest)?,
        })
    }

    pub fn new_copy_file<E>(
        src: &Path,
        dest: &Path,
        transform_path: impl Fn(&Path) -> Result<PathBuf, E>,
    ) -> Result<Self, E> {
        let dest = append_path(dest, src, false);
        Ok(Action::CopyFile {
            src: src.to_owned(),
            dest: transform_path(&dest)?,
        })
    }

    pub fn new_write_template<E>(
        src: &Path,
        dest: &Path,
        transform_path: impl Fn(&Path) -> Result<PathBuf, E>,
    ) -> Result<Self, E> {
        let dest = append_path(dest, src, true);
        Ok(Action::WriteTemplate {
            src: src.to_owned(),
            dest: transform_path(&dest)?,
        })
    }

    pub fn dest(&self) -> &Path {
        use self::Action::*;
        match self {
            CreateDirectory { dest }
            | CopyFile { dest, .. }
            | WriteTemplate { dest, .. } => &dest,
        }
    }
}

fn append_path(base: &Path, other: &Path, strip_extension: bool) -> PathBuf {
    let tail = match strip_extension {
        true => other.file_stem().unwrap(),
        false => other.file_name().unwrap(),
    };
    base.join(tail)
}

fn file_action<E>(
    src: &Path,
    dest: &Path,
    transform_path: impl Fn(&Path) -> Result<PathBuf, E>,
) -> Result<Action, E> {
    let is_template = src
        .extension()
        .map(|ext| ext == TEMPLATE_EXT)
        .unwrap_or(false);
    if is_template {
        Action::new_write_template(src, dest, transform_path)
    } else {
        Action::new_copy_file(src, dest, transform_path)
    }
}

#[derive(Debug, From)]
pub enum TraversalError<E: fmt::Debug> {
    ReadDirectoryError(io::Error),
    ReadEntryError(io::Error),
    TransformPathError(E),
}

fn traverse_dir<E: fmt::Debug>(
    src: &Path,
    dest: &Path,
    transform_path: &impl Fn(&Path) -> Result<PathBuf, E>,
    actions: &mut VecDeque<Action>,
) -> Result<(), TraversalError<E>> {
    if src.is_file() {
        actions.push_back(file_action(src, dest, transform_path)?);
    } else {
        actions.push_front(Action::new_create_directory(dest, transform_path)?);
        for entry in fs::read_dir(src).map_err(TraversalError::ReadDirectoryError)? {
            let path = entry.map_err(TraversalError::ReadEntryError)?.path();
            if path.is_dir() {
                traverse_dir(
                    &path,
                    &append_path(dest, &path, false),
                    transform_path,
                    actions,
                )?;
            } else {
                actions.push_back(file_action(&path, dest, transform_path)?);
            }
        }
    }
    Ok(())
}

pub fn traverse<E: fmt::Debug>(
    src: impl AsRef<Path>,
    dest: impl AsRef<Path>,
    transform_path: impl Fn(&Path) -> Result<PathBuf, E>,
) -> Result<VecDeque<Action>, TraversalError<E>> {
    let src = src.as_ref();
    let dest = dest.as_ref();
    let mut actions = VecDeque::new();
    traverse_dir(src, dest, &transform_path, &mut actions).map(|_| actions)
}
