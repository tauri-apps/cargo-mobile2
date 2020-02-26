use std::{
    fmt::{self, Display},
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};
use zip::{result::ZipError, write::FileOptions, ZipWriter};

#[derive(Debug)]
pub enum Error {
    CreateFailed { tried: PathBuf, cause: io::Error },
    DirReadFailed { tried: PathBuf, cause: io::Error },
    DirEntryFailed { tried: PathBuf, cause: io::Error },
    AddFailed { tried: PathBuf, cause: ZipError },
    ReadFailed { tried: PathBuf, cause: io::Error },
    WriteFailed { tried: PathBuf, cause: io::Error },
    FinishFailed(ZipError),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateFailed { tried, cause } => {
                write!(f, "Failed to create zip file {:?}: {}", tried, cause)
            }
            Self::DirReadFailed { tried, cause } => {
                write!(f, "Failed to read bundle directory {:?}: {}", tried, cause)
            }
            Self::DirEntryFailed { tried, cause } => write!(
                f,
                "Failed to get entry in bundle directory  {:?}: {}",
                tried, cause
            ),
            Self::AddFailed { tried, cause } => {
                write!(f, "Failed to add to zip file {:?}: {}", tried, cause)
            }
            Self::ReadFailed { tried, cause } => {
                write!(f, "Failed to read bundle file {:?}: {}", tried, cause)
            }
            Self::WriteFailed { tried, cause } => write!(
                f,
                "Failed to write contents of bundle file {:?} to zip: {}",
                tried, cause
            ),
            Self::FinishFailed(err) => write!(f, "Failed to finish zip: {}", err),
        }
    }
}

fn traverse(
    writer: &mut ZipWriter<&mut File>,
    options: FileOptions,
    dir: &Path,
    prefix: &Path,
) -> Result<(), Error> {
    for entry in fs::read_dir(dir).map_err(|cause| Error::DirReadFailed {
        tried: dir.to_owned(),
        cause,
    })? {
        let entry = entry.map_err(|cause| Error::DirEntryFailed {
            tried: dir.to_owned(),
            cause,
        })?;
        let path = entry.path();
        let name = path.strip_prefix(prefix).unwrap();
        if path.is_file() {
            log::info!("adding file {:?} to archive as {:?}", path, name);
            writer
                .start_file_from_path(name, options)
                .map_err(|cause| Error::AddFailed {
                    tried: name.to_owned(),
                    cause,
                })?;
            let bytes = fs::read(&path).map_err(|cause| Error::ReadFailed {
                tried: path.to_owned(),
                cause,
            })?;
            writer
                .write_all(&bytes)
                .map_err(|cause| Error::WriteFailed {
                    tried: path.to_owned(),
                    cause,
                })?;
        } else if name.as_os_str().len() != 0 {
            log::info!("adding directory {:?} to archive as {:?}", path, name);
            traverse(writer, options, &path, prefix)?;
        }
    }
    Ok(())
}

pub fn zip(bundle_root: &Path, bundle_path: &Path) -> Result<PathBuf, Error> {
    let zip_path = PathBuf::from(format!("{}.zip", bundle_path.display()));
    let mut file = File::create(&zip_path).map_err(|cause| Error::CreateFailed {
        tried: zip_path.clone(),
        cause,
    })?;
    let mut writer = ZipWriter::new(&mut file);
    let options = FileOptions::default();
    traverse(&mut writer, options, bundle_path, bundle_root)?;
    writer.finish().map_err(Error::FinishFailed)?;
    Ok(zip_path)
}
