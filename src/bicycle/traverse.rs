use std::{
    collections::VecDeque,
    error::Error as StdError,
    fmt::{Debug, Display},
    fs, io,
    path::{Path, PathBuf},
};
use thiserror::Error;

/// Instruction for performing a filesystem action or template processing.
#[derive(Debug)]
pub enum Action {
    /// Specifies to create a new directory at `dest`.
    CreateDirectory { dest: PathBuf },
    /// Specifies to copy the file at `src` to `dest`.
    CopyFile { src: PathBuf, dest: PathBuf },
    /// Specifies to render the template at `src` to `dest`.
    WriteTemplate { src: PathBuf, dest: PathBuf },
}

impl Action {
    /// Create a new [`Action::CreateDirectory`].
    /// Uses `transform_path` to transform `dest`.
    pub fn new_create_directory<E>(
        dest: &Path,
        transform_path: impl Fn(&Path) -> Result<PathBuf, E>,
    ) -> Result<Self, E> {
        Ok(Self::CreateDirectory {
            dest: transform_path(dest)?,
        })
    }

    /// Create a new [`Action::CopyFile`].
    /// Uses `transform_path` to transform `dest`.
    pub fn new_copy_file<E>(
        src: &Path,
        dest: &Path,
        transform_path: impl Fn(&Path) -> Result<PathBuf, E>,
    ) -> Result<Self, E> {
        let dest = append_path(dest, src, false);
        Ok(Self::CopyFile {
            src: src.to_owned(),
            dest: transform_path(&dest)?,
        })
    }

    /// Create a new [`Action::WriteTemplate`].
    /// Uses `transform_path` to transform `dest`.
    pub fn new_write_template<E>(
        src: &Path,
        dest: &Path,
        transform_path: impl Fn(&Path) -> Result<PathBuf, E>,
    ) -> Result<Self, E> {
        let dest = append_path(dest, src, true);
        Ok(Self::WriteTemplate {
            src: src.to_owned(),
            dest: transform_path(&dest)?,
        })
    }

    pub fn is_create_directory(&self) -> bool {
        matches!(self, Self::CreateDirectory { .. })
    }

    pub fn is_copy_file(&self) -> bool {
        matches!(self, Self::CopyFile { .. })
    }

    pub fn is_write_template(&self) -> bool {
        matches!(self, Self::WriteTemplate { .. })
    }

    /// Gets the destination of any [`Action`] variant.
    pub fn dest(&self) -> &Path {
        match self {
            Self::CreateDirectory { dest }
            | Self::CopyFile { dest, .. }
            | Self::WriteTemplate { dest, .. } => dest,
        }
    }
}

fn append_path(base: &Path, other: &Path, strip_extension: bool) -> PathBuf {
    let tail = if strip_extension {
        other.file_stem().unwrap()
    } else {
        other.file_name().unwrap()
    };
    base.join(tail)
}

fn file_action<E>(
    src: &Path,
    dest: &Path,
    transform_path: impl Fn(&Path) -> Result<PathBuf, E>,
    template_ext: Option<&str>,
) -> Result<Action, E> {
    let is_template = template_ext
        .and_then(|template_ext| src.extension().filter(|ext| *ext == template_ext))
        .is_some();
    if is_template {
        Action::new_write_template(src, dest, transform_path)
    } else {
        Action::new_copy_file(src, dest, transform_path)
    }
}

/// An error encountered when traversing a file tree.
#[derive(Debug, Error)]
pub enum TraversalError<E: Debug + Display + StdError + 'static = super::RenderingError> {
    /// Failed to get directory listing.
    #[error("Failed to read directory at {path:?}: {cause}")]
    DirectoryRead {
        path: PathBuf,
        #[source]
        cause: io::Error,
    },
    /// Failed to inspect entry from directory listing.
    #[error("Failed to read directory entry in {dir:?}: {cause}")]
    EntryRead {
        dir: PathBuf,
        #[source]
        cause: io::Error,
    },
    /// Failed to transform path.
    #[error("Failed to transform path at {path:?}: {cause}")]
    PathTransform {
        path: PathBuf,
        #[source]
        cause: E,
    },
}

fn traverse_dir<E: Debug + Display + StdError>(
    src: &Path,
    dest: &Path,
    transform_path: &impl Fn(&Path) -> Result<PathBuf, E>,
    template_ext: Option<&str>,
    actions: &mut VecDeque<Action>,
) -> Result<(), TraversalError<E>> {
    if src.is_file() {
        actions.push_back(
            file_action(src, dest, transform_path, template_ext).map_err(|cause| {
                TraversalError::PathTransform {
                    path: dest.to_owned(),
                    cause,
                }
            })?,
        );
    } else {
        actions.push_front(Action::new_create_directory(dest, transform_path).map_err(
            |cause| TraversalError::PathTransform {
                path: dest.to_owned(),
                cause,
            },
        )?);
        for entry in fs::read_dir(src).map_err(|cause| TraversalError::DirectoryRead {
            path: src.to_owned(),
            cause,
        })? {
            let path = entry
                .map_err(|cause| TraversalError::EntryRead {
                    dir: src.to_owned(),
                    cause,
                })?
                .path();
            if path.is_dir() {
                traverse_dir(
                    &path,
                    &append_path(dest, &path, false),
                    transform_path,
                    template_ext,
                    actions,
                )?;
            } else {
                actions.push_back(
                    file_action(&path, dest, transform_path, template_ext).map_err(|cause| {
                        TraversalError::PathTransform {
                            path: path.to_owned(),
                            cause,
                        }
                    })?,
                );
            }
        }
    }
    Ok(())
}

/// Traverse file tree at `src` to generate an [`Action`] list.
/// The [`Action`] list specifies how to generate the `src` file tree at `dest`,
/// and can be executed by [`Bicycle::process_actions`](super::Bicycle::process_actions).
///
/// File tree contents are interpreted as follows:
/// - Each directory in the file tree generates an [`Action::CreateDirectory`].
///   Directories are traversed recursively.
/// - Each file that doesn't end in `template_ext` generates an [`Action::CopyFile`].
/// - Each file that ends in `template_ext` generates an [`Action::WriteTemplate`].
///
/// `transform_path` is used to post-process destination path strings.
/// [`Bicycle::transform_path`](super::Bicycle::transform_path) is one possible implementation.
pub fn traverse<E: Debug + Display + StdError>(
    src: impl AsRef<Path>,
    dest: impl AsRef<Path>,
    transform_path: impl Fn(&Path) -> Result<PathBuf, E>,
    template_ext: Option<&str>,
) -> Result<VecDeque<Action>, TraversalError<E>> {
    let src = src.as_ref();
    let dest = dest.as_ref();
    let mut actions = VecDeque::new();
    traverse_dir(src, dest, &transform_path, template_ext, &mut actions).map(|_| actions)
}

/// Pass this to `traverse` if you don't want any path transformation at all.
pub fn no_transform(path: &Path) -> Result<PathBuf, std::convert::Infallible> {
    Ok(path.to_owned())
}

/// `Some("hbs")`. Pass this to `traverse` to get the same template
/// identification behavior as `Bicycle::process`.
pub static DEFAULT_TEMPLATE_EXT: Option<&'static str> = Some("hbs");
