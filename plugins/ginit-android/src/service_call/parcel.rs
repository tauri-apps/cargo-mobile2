use ginit_core::exports::once_cell_regex::regex;
use std::fmt::{self, Display};

#[derive(Debug)]
pub enum Error {
    NotMatched,
    WrongCaptureCount {
        expected: usize,
        found: usize,
    },
    FirstByteInvalid {
        raw: String,
        cause: std::num::ParseIntError,
    },
    SecondByteInvalid {
        raw: String,
        cause: std::num::ParseIntError,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotMatched => write!(f, "The output contained no matches."),
            Self::WrongCaptureCount { expected, found } => write!(
                f,
                "Expected {} capture groups, but found {}.",
                expected, found
            ),
            Self::FirstByteInvalid { raw, cause } => {
                write!(f, "First byte {:?} couldn't be parsed: {}", raw, cause)
            }
            Self::SecondByteInvalid { raw, cause } => {
                write!(f, "Second byte {:?} couldn't be parsed: {}", raw, cause)
            }
        }
    }
}

#[derive(Debug)]
pub struct Parcel {
    data: u16,
}

impl Parcel {
    pub fn from_output(output: &str) -> Result<Self, Error> {
        let re = regex!(r"Result: Parcel\((\d{8}) (\d{8})\s+'(.*)'\)");
        let caps = re.captures(output).ok_or_else(|| Error::NotMatched)?;
        const CAP_COUNT: usize = 3;
        if caps.len() == CAP_COUNT {
            let first: u8 = {
                let raw = &caps[1];
                raw.parse().map_err(|cause| Error::FirstByteInvalid {
                    raw: raw.to_owned(),
                    cause,
                })?
            };
            let second: u8 = {
                let raw = &caps[2];
                raw.parse().map_err(|cause| Error::SecondByteInvalid {
                    raw: raw.to_owned(),
                    cause,
                })?
            };
            Ok(Self {
                data: u16::from_be_bytes([first, second]),
            })
        } else {
            Err(Error::WrongCaptureCount {
                expected: CAP_COUNT,
                found: caps.len(),
            })
        }
    }

    pub fn get_bool(&self) -> bool {
        self.data != 0
    }
}
