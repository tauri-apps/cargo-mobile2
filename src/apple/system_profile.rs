use once_cell_regex::regex;
use std::str;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("`system_profiler` call failed: {0}")]
    SystemProfilerFailed(#[from] bossy::Error),
    #[error("`system_profiler` output contained invalid UTF-8: {0}")]
    OutputInvalidUtf8(#[from] str::Utf8Error),
    #[error("No version number was found within the `SPDeveloperToolsDataType` data: {data:?}")]
    VersionNotMatched { data: String },
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
        let version_re = regex!(r"\bVersion: (?P<major>\d+)\.(?P<minor>\d+)\b");
        // The `-xml` flag can be used to get this info in plist format, but
        // there don't seem to be any high quality plist crates, and parsing
        // XML sucks, we'll be lazy for now.
        let output = bossy::Command::impure("system_profiler")
            .with_arg("SPDeveloperToolsDataType")
            .run_and_wait_for_output()
            .map_err(Error::SystemProfilerFailed)?;
        let text = output.stdout_str().map_err(Error::OutputInvalidUtf8)?;
        let caps = version_re
            .captures(text)
            .ok_or_else(|| Error::VersionNotMatched {
                data: text.to_owned(),
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
