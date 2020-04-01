use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Clobbering {
    Forbid,
    Allow,
}

impl Default for Clobbering {
    fn default() -> Self {
        Clobbering::Forbid
    }
}

impl Clobbering {
    pub fn is_allowed(self) -> bool {
        self == Clobbering::Allow
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpenIn {
    Nothing,
    Editor,
}

impl Default for OpenIn {
    fn default() -> Self {
        OpenIn::Nothing
    }
}

impl OpenIn {
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
    pub fn is_debug(self) -> bool {
        self == Profile::Debug
    }

    pub fn is_release(self) -> bool {
        self == Profile::Release
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Profile::Debug => "debug",
            Profile::Release => "release",
        }
    }
}
