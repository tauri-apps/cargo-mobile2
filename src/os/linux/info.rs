use crate::os::Info;
use once_cell_regex::regex;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to read {path:?}: {source}")]
    ReadFailed {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Failed to find `NAME` in {path:?}: {text:?}")]
    NameMissing { path: PathBuf, text: String },
    #[error("Failed to find `VERSION` in {path:?}: {text:?}")]
    VersionMissing { path: PathBuf, text: String },
}

pub fn check() -> Result<Info, Error> {
    // Doing this robustly seems like a huge nightmare, since not all distros
    // use this path or even the same format... we'll punt on that for now.
    // https://grep.app/search?q=os-release&case=true
    let path = "/etc/os-release";
    let release = std::fs::read_to_string(path).map_err(|source| Error::ReadFailed {
        path: path.into(),
        source,
    })?;
    // I'll remain optimistic that these regexes won't choke on some unknown
    // edge-cases... (as always)
    let name = regex!(r#"\bNAME="?(.*)\b"#)
        .captures(&release)
        .map(|caps| caps[1].to_owned())
        .ok_or_else(|| Error::NameMissing {
            path: path.into(),
            text: release.to_owned(),
        })?;
    let version = regex!(r#"\bVERSION="?(.*)\b"#)
        .captures(&release)
        .map(|caps| caps[1].to_owned())
        .ok_or_else(|| Error::VersionMissing {
            path: path.into(),
            text: release.to_owned(),
        })?;
    Ok(Info { name, version })
}
