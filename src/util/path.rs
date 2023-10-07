use path_abs::PathAbs;
use std::{
    fmt::{self, Display},
    io,
    path::{Component, Path, PathBuf},
    time::SystemTime,
};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Failed to get user's home directory!")]
pub struct NoHomeDir;

pub fn home_dir() -> Result<PathBuf, NoHomeDir> {
    home::home_dir().ok_or(NoHomeDir)
}

pub fn expand_home(path: impl AsRef<Path>) -> Result<PathBuf, NoHomeDir> {
    let home = home_dir()?;
    let path = path.as_ref();
    if let Ok(path) = path.strip_prefix("~") {
        Ok(home.join(path))
    } else {
        Ok(path.to_owned())
    }
}

#[derive(Debug, Error)]
pub enum ContractHomeError {
    #[error(transparent)]
    NoHomeDir(#[from] NoHomeDir),
    #[error("User's home directory path wasn't valid UTF-8.")]
    HomeInvalidUtf8,
    #[error("Supplied path wasn't valid UTF-8.")]
    PathInvalidUtf8,
}

pub fn contract_home(path: impl AsRef<Path>) -> Result<String, ContractHomeError> {
    let path = path
        .as_ref()
        .to_str()
        .ok_or(ContractHomeError::PathInvalidUtf8)?;
    #[cfg(not(windows))]
    {
        let home = home_dir()?;
        let home = home.to_str().ok_or(ContractHomeError::HomeInvalidUtf8)?;
        Ok(path.replace(home, "~"))
    }
    #[cfg(windows)]
    {
        Ok(path.to_owned())
    }
}

pub fn install_dir() -> Result<PathBuf, NoHomeDir> {
    let dir_name = concat!(".", env!("CARGO_PKG_NAME"));
    std::env::var("CARGO_HOME")
        .map(|p| PathBuf::from(p).join(dir_name))
        .or_else(|_| home_dir().map(|home| home.join(".cargo").join(dir_name)))
}

pub fn checkouts_dir() -> Result<PathBuf, NoHomeDir> {
    install_dir().map(|install_dir| install_dir.join("checkouts"))
}

pub fn tools_dir() -> Result<PathBuf, NoHomeDir> {
    install_dir().map(|install_dir| install_dir.join("tools"))
}

#[derive(Debug)]
pub struct PathNotPrefixed {
    path: PathBuf,
    prefix: PathBuf,
}

impl Display for PathNotPrefixed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Path {:?} didn't have prefix {:?}.",
            self.path, self.prefix
        )
    }
}

pub fn prefix_path(root: impl AsRef<Path>, path: impl AsRef<Path>) -> PathBuf {
    let root = root.as_ref();
    let path = path.as_ref();
    let is_verbatim = if let Some(Component::Prefix(prefix)) = root.components().next() {
        prefix.kind().is_verbatim()
    } else {
        false
    };
    if !is_verbatim {
        return root.join(path);
    }
    let mut buf = root.components().collect::<Vec<_>>();
    for component in path.components() {
        match component {
            Component::RootDir => {
                buf.truncate(1);
                buf.push(component);
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if buf.last().is_some() {
                    buf.pop();
                }
            }
            _ => buf.push(component),
        };
    }
    buf.into_iter().collect()
}

pub fn unprefix_path(
    root: impl AsRef<Path>,
    path: impl AsRef<Path>,
) -> Result<PathBuf, PathNotPrefixed> {
    let root = root.as_ref();
    let path = path.as_ref();
    path.strip_prefix(root)
        .map(|path| path.to_owned())
        .map_err(|_| PathNotPrefixed {
            path: path.to_owned(),
            prefix: root.to_owned(),
        })
}

fn common_root(abs_src: &Path, abs_dest: &Path) -> PathBuf {
    let mut dest_root = abs_dest.to_owned();
    loop {
        if abs_src.starts_with(&dest_root) {
            return dest_root;
        } else if !dest_root.pop() {
            unreachable!("`abs_src` and `abs_dest` have no common root");
        }
    }
}

/// Transforms `abs_path` to be relative to `abs_relative_to`.
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
pub enum NormalizationError {
    CanonicalizationFailed {
        path: PathBuf,
        cause: io::Error,
    },
    PathAbsFailed {
        path: PathBuf,
        cause: path_abs::Error,
    },
}

impl Display for NormalizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CanonicalizationFailed { path, cause } => write!(
                f,
                "Failed to canonicalize existing path {:?}: {}",
                path, cause
            ),
            Self::PathAbsFailed { path, cause } => write!(
                f,
                "Failed to normalize non-existent path {:?}: {}",
                path, cause
            ),
        }
    }
}

pub fn normalize_path(path: impl AsRef<Path>) -> Result<PathBuf, NormalizationError> {
    let path = path.as_ref();
    if path.exists() {
        path.canonicalize()
            .map_err(|cause| NormalizationError::CanonicalizationFailed {
                path: path.to_owned(),
                cause,
            })
    } else {
        PathAbs::new(path)
            .map_err(|cause| NormalizationError::PathAbsFailed {
                path: path.to_owned(),
                cause,
            })
            .map(|abs| abs.as_path().to_owned())
    }
}

pub fn under_root(
    path: impl AsRef<Path>,
    root: impl AsRef<Path>,
) -> Result<bool, NormalizationError> {
    let root = dunce::simplified(root.as_ref());
    normalize_path(root.join(path)).map(|norm| {
        let norm = dunce::simplified(&norm);
        norm.starts_with(dunce::simplified(root))
    })
}

pub fn last_modified(first: PathBuf, second: PathBuf) -> PathBuf {
    let first_modified = first
        .metadata()
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let second_modified = second
        .metadata()
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);
    match first_modified.cmp(&second_modified) {
        std::cmp::Ordering::Less => second,
        std::cmp::Ordering::Equal => first,
        std::cmp::Ordering::Greater => first,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rstest::rstest;

    #[rstest(root, path, result,
        // UNIX
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        case(
            "/home/user/cargo-mobile2-project/gen/android/cargo-mobile2-project",
            "app/build/outputs/apk/arm64/debug/app-arm64-debug.apk",
            "/home/user/cargo-mobile2-project/gen/android/cargo-mobile2-project/app/build/outputs/apk/arm64/debug/app-arm64-debug.apk"
        ),
        // UNIX but the second path contains root
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        case(
            "/home/user/cargo-mobile2-project/gen/android/cargo-mobile2-project",
            "/home/other/project/gen/android/app/build/outputs/apk/arm64/debug/app-arm64-debug.apk",
            "/home/other/project/gen/android/app/build/outputs/apk/arm64/debug/app-arm64-debug.apk"
        ),
        // Windows UNC
        #[cfg(windows)]
        case(
            "\\\\?\\C:\\Users\\user\\cargo-mobile2-project\\gen\\android\\cargo-mobile2-project",
            "app\\..\\app\\build\\outputs\\.\\apk\\arm64\\debug\\app-arm64-debug.apk",
            "\\\\?\\C:\\Users\\user\\cargo-mobile2-project\\gen\\android\\cargo-mobile2-project\\app\\build\\outputs\\apk\\arm64\\debug\\app-arm64-debug.apk"
        ),
        // Windows legacy
        #[cfg(windows)]
        case (
            "D:\\Users\\user\\cargo-mobile2-project\\gen\\android\\cargo-mobile2-project",
            "app\\build\\outputs\\apk\\arm64\\debug\\app-arm64-debug.apk",
            "D:\\Users\\user\\cargo-mobile2-project\\gen\\android\\cargo-mobile2-project\\app\\build\\outputs\\apk\\arm64\\debug\\app-arm64-debug.apk"
        )
    )]
    fn test_prefix_path(root: impl AsRef<Path>, path: impl AsRef<Path>, result: &str) {
        assert_eq!(prefix_path(root, path), PathBuf::from(result));
    }
}
