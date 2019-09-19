use path_abs::PathAbs;
use std::{
    fmt, io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum UnprefixPathError {
    PathNotPrefixed,
}

impl fmt::Display for UnprefixPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnprefixPathError::PathNotPrefixed => write!(
                f,
                "Attempted to remove the project path prefix from a path that wasn't in the project."
            ),
        }
    }
}

pub fn prefix_path(root: impl AsRef<Path>, path: impl AsRef<Path>) -> PathBuf {
    root.as_ref().join(path)
}

pub fn unprefix_path(
    root: impl AsRef<Path>,
    path: impl AsRef<Path>,
) -> Result<PathBuf, UnprefixPathError> {
    path.as_ref()
        .strip_prefix(root)
        .map(|path| path.to_owned())
        .map_err(|_| UnprefixPathError::PathNotPrefixed)
}

fn common_root(abs_src: &Path, abs_dest: &Path) -> PathBuf {
    let mut dest_root = abs_dest.to_owned();
    loop {
        if abs_src.starts_with(&dest_root) {
            return dest_root;
        } else {
            if !dest_root.pop() {
                unreachable!("`abs_src` and `abs_dest` have no common root");
            }
        }
    }
}

pub fn relativize_path(abs_path: impl AsRef<Path>, abs_relative_to: impl AsRef<Path>) -> PathBuf {
    let (abs_path, abs_relative_to) = (abs_path.as_ref(), abs_relative_to.as_ref());
    assert!(abs_path.is_absolute());
    assert!(abs_relative_to.is_absolute());
    let (path, relative_to) = {
        let common_root = common_root(abs_path, abs_relative_to);
        let path = abs_path.strip_prefix(&common_root).unwrap();
        let relative_to = abs_relative_to.strip_prefix(&common_root).unwrap();
        (path, relative_to)
    };
    let mut rel_path = PathBuf::new();
    for _ in 0..relative_to.iter().count() {
        rel_path.push("..");
    }
    let rel_path = rel_path.join(path);
    log::info!(
        "{:?} relative to {:?} is {:?}",
        abs_path,
        abs_relative_to,
        rel_path
    );
    rel_path
}

#[derive(Debug)]
pub enum NormalizationErrorCause {
    CanonicalizationFailed(io::Error),
    PathAbsFailed(path_abs::Error),
}

impl fmt::Display for NormalizationErrorCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CanonicalizationFailed(err) => {
                write!(f, "Failed to canonicalize existing path: {}", err)
            }
            Self::PathAbsFailed(err) => write!(f, "Failed to normalize non-existant path: {}", err),
        }
    }
}

#[derive(Debug)]
pub struct NormalizationError {
    path: PathBuf,
    cause: NormalizationErrorCause,
}

impl fmt::Display for NormalizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Failed to normalize path {:?}: {}",
            self.path, self.cause
        )
    }
}

pub fn normalize_path(path: impl AsRef<Path>) -> Result<PathBuf, NormalizationError> {
    let path = path.as_ref();
    if path.exists() {
        path.canonicalize().map_err(|cause| NormalizationError {
            path: path.to_owned(),
            cause: NormalizationErrorCause::CanonicalizationFailed(cause),
        })
    } else {
        PathAbs::new(path)
            .map_err(|cause| NormalizationError {
                path: path.to_owned(),
                cause: NormalizationErrorCause::PathAbsFailed(cause),
            })
            .map(|abs| abs.as_path().to_owned())
    }
}
