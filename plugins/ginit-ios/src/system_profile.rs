use ginit_core::exports::{bossy, once_cell_regex::regex};
use std::{
    fmt::{self, Display},
    str,
};

#[derive(Debug)]
pub enum Error {
    SystemProfilerFailed(bossy::Error),
    OutputInvalidUtf8(str::Utf8Error),
    VersionNotMatched {
        data: String,
    },
    MajorVersionInvalid {
        major: String,
        cause: std::num::ParseIntError,
    },
    MinorVersionInvalid {
        minor: String,
        cause: std::num::ParseIntError,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SystemProfilerFailed(err) => write!(f, "`system_profiler` call failed: {}", err),
            Self::OutputInvalidUtf8(err) => write!(
                f,
                "`system_profiler` output contained invalid UTF-8: {}",
                err
            ),
            Self::VersionNotMatched { data } => write!(
                f,
                "No version number was found within the `SPDeveloperToolsDataType` data: {:?}",
                data
            ),
            Self::MajorVersionInvalid { major, cause } => write!(
                f,
                "The major version {:?} wasn't a valid number: {}",
                major, cause
            ),
            Self::MinorVersionInvalid { minor, cause } => write!(
                f,
                "The minor version {:?} wasn't a valid number: {}",
                minor, cause
            ),
        }
    }
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
                .map_err(|cause| Error::MajorVersionInvalid {
                    major: raw.to_owned(),
                    cause,
                })?
        };
        let minor = {
            let raw = &caps["minor"];
            raw.parse::<u32>()
                .map_err(|cause| Error::MinorVersionInvalid {
                    minor: raw.to_owned(),
                    cause,
                })?
        };
        Ok(Self {
            version: (major, minor),
        })
    }
}
