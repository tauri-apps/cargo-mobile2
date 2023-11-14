use crate::util::{VersionTriple, VersionTripleError};
use serde::{ser::Serializer, Serialize};
use std::{
    fmt::{self, Debug, Display},
    str::FromStr,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VersionNumberError {
    #[error("Failed to parse version triple.")]
    VersionTripleInvalid(#[from] VersionTripleError),
    #[error("Failed to parse extra version from {version:?}: {source}")]
    ExtraVersionInvalid {
        version: String,
        source: std::num::ParseIntError,
    },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct VersionNumber {
    pub triple: VersionTriple,
    pub extra: Option<Vec<u32>>,
}

impl Display for VersionNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.triple)?;
        if let Some(extra) = &self.extra {
            for number in extra {
                write!(f, ".{}", number)?;
            }
        }
        Ok(())
    }
}

impl Serialize for VersionNumber {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl FromStr for VersionNumber {
    type Err = VersionNumberError;

    fn from_str(v: &str) -> Result<Self, Self::Err> {
        match v.split('.').count() {
            1..=3 => {
                let triple = VersionTriple::from_str(v)?;
                Ok(Self {
                    triple,
                    extra: None,
                })
            }
            // Even when splitting a string that does not contain the delimeter, we should always get at least 1 split
            // (the full string, which could be the empty string)
            0 => unreachable!(),
            _ => {
                let mut s = v.split('.');
                let triple = VersionTriple::from_split(&mut s, v)?;
                let extra = Some(
                    s.map(|s| {
                        s.parse()
                            .map_err(|source| VersionNumberError::ExtraVersionInvalid {
                                version: v.to_owned(),
                                source,
                            })
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                );
                Ok(Self { triple, extra })
            }
        }
    }
}

impl VersionNumber {
    pub fn new_from_triple(triple: VersionTriple) -> Self {
        Self {
            triple,
            extra: None,
        }
    }

    pub const fn new(triple: VersionTriple, extra: Option<Vec<u32>>) -> Self {
        Self { triple, extra }
    }

    pub fn push_extra(&mut self, number: u32) {
        self.extra.get_or_insert_with(Default::default).push(number);
    }
}
