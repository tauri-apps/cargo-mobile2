use into_result::{command::CommandError, IntoResult as _};
use regex::Regex;
use std::{process::Command, str};

#[derive(Debug)]
pub enum DeveloperToolsError {
    SystemProfilerFailed(CommandError),
    OutputWasInvalidUtf8(str::Utf8Error),
    VersionNotMatched,
    VersionComponentNotNumeric(std::num::ParseIntError),
}

// There's a bunch more info available, but the version is all we need for now.
#[derive(Debug)]
pub struct DeveloperTools {
    pub version: (u32, u32),
}

impl DeveloperTools {
    pub fn new() -> Result<Self, DeveloperToolsError> {
        lazy_static::lazy_static! {
            static ref VERSION_RE: Regex = Regex::new(r#"\bVersion: (\d+)\.(\d+)\b"#).unwrap();
        }
        let bytes = Command::new("system_profiler")
            .arg("SPDeveloperToolsDataType")
            .output()
            .into_result()
            .map_err(DeveloperToolsError::SystemProfilerFailed)
            .map(|out| out.stdout)?;
        let text = str::from_utf8(&bytes).map_err(DeveloperToolsError::OutputWasInvalidUtf8)?;
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
                            .map_err(DeveloperToolsError::VersionComponentNotNumeric)
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .ok_or(DeveloperToolsError::VersionNotMatched)??;
        Ok(Self {
            version: (components[0], components[1]),
        })
    }
}
