use serde::{Deserialize, Serialize};
#[cfg(feature = "cli")]
use structopt::clap::arg_enum;

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

    pub fn suffixes(&self) -> &[&str] {
        match self {
            Profile::Debug => &["debug"],
            Profile::Release => &["release", "release-unsigned"],
        }
    }
}

/// Android device logging filter level, used as an argument for run
#[cfg(not(feature = "cli"))]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FilterLevel {
    Error,
    Warn,
    Info,
    Debug,
    Verbose,
}

#[cfg(feature = "cli")]
arg_enum! {
    /// Android device logging filter level, used as an argument for run
    #[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub enum FilterLevel {
        Error,
        Warn,
        Info,
        Debug,
        Verbose,
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
