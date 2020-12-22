use crate::util;
use once_cell_regex::regex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    SystemProfilerFailed(#[from] util::RunAndSearchError),
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
        util::run_and_search(
            &mut bossy::Command::impure_parse("system_profiler SPDeveloperToolsDataType"),
            regex!(r"\bVersion: (?P<major>\d+)\.(?P<minor>\d+)\b"),
            |_text, caps| {
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
            },
        )
        .map_err(Error::SystemProfilerFailed)?
    }
}
