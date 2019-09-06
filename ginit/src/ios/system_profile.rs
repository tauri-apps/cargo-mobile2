use into_result::{command::CommandError, IntoResult as _};
use regex::Regex;
use std::{fmt, process::Command, str};

#[derive(Debug)]
pub enum Error {
    SystemProfilerFailed(CommandError),
    OutputWasInvalidUtf8(str::Utf8Error),
    VersionNotMatched,
    VersionComponentNotNumeric(std::num::ParseIntError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::SystemProfilerFailed(err) => write!(f, "`system_profiler` call failed: {}", err),
            Error::OutputWasInvalidUtf8(err) => write!(
                f,
                "`system_profiler` output contained invalid UTF-8: {}",
                err
            ),
            Error::VersionNotMatched => write!(
                f,
                "No version number was found within the `SPDeveloperToolsDataType` data."
            ),
            Error::VersionComponentNotNumeric(err) => write!(
                f,
                "The version contained something that wasn't a valid number: {}",
                err
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
        lazy_static::lazy_static! {
            static ref VERSION_RE: Regex = Regex::new(r#"\bVersion: (\d+)\.(\d+)\b"#).unwrap();
        }
        // The `-xml` flag can be used to get this info in plist format, but
        // there don't seem to be any high quality plist crates, and parsing
        // XML sucks, we'll be lazy for now.
        let bytes = Command::new("system_profiler")
            .arg("SPDeveloperToolsDataType")
            .output()
            .into_result()
            .map_err(Error::SystemProfilerFailed)
            .map(|out| out.stdout)?;
        let text = str::from_utf8(&bytes).map_err(Error::OutputWasInvalidUtf8)?;
        let components = VERSION_RE
            .captures_iter(text)
            .next()
            .map(|caps| {
                debug_assert_eq!(caps.len(), 2);
                caps.iter()
                    .skip(1)
                    .map(|component| {
                        component
                            .unwrap()
                            .as_str()
                            .parse::<u32>()
                            .map_err(Error::VersionComponentNotNumeric)
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .ok_or(Error::VersionNotMatched)??;
        Ok(Self {
            version: (components[0], components[1]),
        })
    }
}
