use crate::util;
use once_cell_regex::regex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    SystemProfilerFailed(#[from] util::RunAndSearchError),
    #[error("Xcode doesn't appear to be installed.")]
    XcodeNotInstalled,
    #[error("The major version {major:?} wasn't a valid number: {source}")]
    MajorVersionInvalid {
        major: String,
        source: std::num::ParseIntError,
    },
    #[error("The minor version {minor:?} wasn't a valid number: {source}")]
    MinorVersionInvalid {
        minor: String,
        source: std::num::ParseIntError,
    },
}

// There's a bunch more info available, but the version is all we need for now.
#[derive(Debug)]
pub struct DeveloperTools {
    pub version: (u32, u32),
}

impl DeveloperTools {
    pub fn new() -> Result<Self, Error> {
        // The `-xml` flag can be used to get this info in plist format, but
        // there don't seem to be any high quality plist crates, and parsing
        // XML sucks, we'll be lazy for now.
        let command = duct::cmd("system_profiler", ["SPDeveloperToolsDataType"]).stderr_capture();
        let command_string = format!("{command:?}");
        let output = command.read().map_err(util::RunAndSearchError::from)?;
        if output.is_empty() {
            Err(Error::XcodeNotInstalled)
        } else {
            let caps = regex!(r"\bVersion: (?P<major>\d+)\.(?P<minor>\d+)\b")
                .captures(&output)
                .ok_or_else(|| util::RunAndSearchError::SearchFailed {
                    command: command_string,
                    output: output.to_owned(),
                })?;
            let major = {
                let raw = &caps["major"];
                raw.parse::<u32>()
                    .map_err(|source| Error::MajorVersionInvalid {
                        major: raw.to_owned(),
                        source,
                    })?
            };
            let minor = {
                let raw = &caps["minor"];
                raw.parse::<u32>()
                    .map_err(|source| Error::MinorVersionInvalid {
                        minor: raw.to_owned(),
                        source,
                    })?
            };
            Ok(Self {
                version: (major, minor),
            })
        }
    }
}
