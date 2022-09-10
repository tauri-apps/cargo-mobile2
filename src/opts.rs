use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum NoiseLevel {
    Polite,
    LoudAndProud,
    FranklyQuitePedantic,
}

impl Default for NoiseLevel {
    fn default() -> Self {
        Self::Polite
    }
}

impl NoiseLevel {
    pub fn from_occurrences(occurrences: u64) -> Self {
        match occurrences {
            0 => Self::Polite,
            1 => Self::LoudAndProud,
            _ => Self::FranklyQuitePedantic,
        }
    }

    pub fn polite(self) -> bool {
        matches!(self, Self::Polite)
    }

    pub fn loud(self) -> bool {
        matches!(self, Self::LoudAndProud)
    }

    pub fn pedantic(self) -> bool {
        matches!(self, Self::FranklyQuitePedantic)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Profile {
    Debug,
    Release,
}

impl Profile {
    pub fn from_flag(flag: bool) -> Self {
        if flag {
            Self::Release
        } else {
            Self::Debug
        }
    }

    pub fn debug(self) -> bool {
        matches!(self, Self::Debug)
    }

    pub fn release(self) -> bool {
        matches!(self, Self::Release)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Release => "release",
        }
    }

    pub fn as_str_pascal_case(&self) -> &'static str {
        match self {
            Self::Debug => "Debug",
            Self::Release => "Release",
        }
    }

    pub fn suffix(&self) -> &str {
        match self {
            Profile::Debug => self.as_str(),
            // TODO: how to handle signed APKs?
            Profile::Release => "release-unsigned",
        }
    }
}

/// Android device logging filter level, used as an argument for run
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FilterLevel {
    Error,
    Warn,
    Info,
    Debug,
    Verbose,
}

impl FromStr for FilterLevel {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "error" => Ok(Self::Error),
            "warn" => Ok(Self::Warn),
            "info" => Ok(Self::Info),
            "debug" => Ok(Self::Debug),
            "verbose" => Ok(Self::Verbose),
            _ => Err("unknown filter level"),
        }
    }
}

impl FilterLevel {
    pub fn variants() -> &'static [&'static str] {
        &["error", "warn", "info", "debug", "verbose"]
    }
}

impl FilterLevel {
    /// Filter level for logcat
    pub fn logcat(&self) -> &'static str {
        match self {
            Self::Error => "E",
            Self::Warn => "W",
            Self::Info => "I",
            Self::Debug => "D",
            Self::Verbose => "V",
        }
    }
}
