use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Interactivity {
    Full,
    None,
}

impl Default for Interactivity {
    fn default() -> Self {
        Self::Full
    }
}

impl Interactivity {
    fn auto() -> Self {
        let is_ci = {
            let ci = std::env::var("CI").ok();
            ci.as_deref() == Some("true") || ci.as_deref() == Some("1")
        };
        if is_ci {
            log::info!(
                "env var `CI` is set to `true` or `1`; automatically running in non-interactive mode"
            );
            Self::None
        } else {
            Self::default()
        }
    }

    pub fn from_flag(flag: bool) -> Self {
        if flag {
            Self::None
        } else {
            Self::auto()
        }
    }

    pub fn full(&self) -> bool {
        matches!(self, Self::Full)
    }

    pub fn none(&self) -> bool {
        matches!(self, Self::None)
    }
}

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
pub enum ReinstallDeps {
    Yes,
    No,
}

impl Default for ReinstallDeps {
    fn default() -> Self {
        Self::No
    }
}

impl ReinstallDeps {
    pub fn from_flag(flag: bool) -> Self {
        if flag {
            Self::Yes
        } else {
            Self::No
        }
    }

    pub fn yes(self) -> bool {
        matches!(self, Self::Yes)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpenIn {
    Nothing,
    Editor,
}

impl Default for OpenIn {
    fn default() -> Self {
        Self::Nothing
    }
}

impl OpenIn {
    pub fn from_flag(flag: bool) -> Self {
        if flag {
            Self::Editor
        } else {
            Self::Nothing
        }
    }

    pub fn editor(&self) -> bool {
        matches!(self, Self::Editor)
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
}
