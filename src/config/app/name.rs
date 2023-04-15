use crate::{
    reserved_names::{is_reserved, Reservation},
    util,
};
use heck::{ToKebabCase as _, ToSnekCase as _};
use std::{
    fmt::{self, Display},
    ops::Deref,
};

#[derive(Debug)]
pub enum Invalid {
    Empty,
    NotAscii {
        app_name: String,
        suggested: Option<String>,
    },
    StartsWithDigit {
        app_name: String,
        suggested: Option<String>,
    },
    ReservedKeyword {
        app_name: String,
    },
    ReservedWindows {
        app_name: String,
    },
    ReservedArtifacts {
        app_name: String,
    },
    NotAlphanumericHyphenOrUnderscore {
        app_name: String,
        naughty_chars: Vec<char>,
        suggested: Option<String>,
    },
}

impl Display for Invalid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "The app name can't be empty.")?,
            Self::NotAscii { app_name, .. } => write!(
                f,
                "\"{}\" isn't valid ASCII.",
                app_name,
            )?,
            Self::StartsWithDigit { app_name, .. } => write!(f, "\"{}\" starts with a digit.", app_name)?,
            Self::ReservedKeyword { app_name } => write!(
                f,
                "\"{}\" is a reserved keyword: https://doc.rust-lang.org/reference/keywords.html",
                app_name,
            )?,
            Self::ReservedWindows { app_name } => write!(
                f,
                "\"{}\" is a reserved name on Windows.",
                app_name,
            )?,
            Self::ReservedArtifacts { app_name } => write!(
                f,
                "\"{}\" is reserved by Cargo.",
                app_name,
            )?,
            Self::NotAlphanumericHyphenOrUnderscore { app_name, naughty_chars, .. } => write!(
                f,
                "\"{}\" contains {}, but only lowercase letters, numbers, hyphens, and underscores are allowed.",
                app_name,
                util::list_display(
                    &naughty_chars.iter().map(|c| format!("'{}'", c)).collect::<Vec<_>>()
                ),
            )?,
        }
        if let Some(suggested) = self.suggested() {
            write!(f, " \"{}\" would work, if you'd like!", suggested)?;
        }
        Ok(())
    }
}

impl Invalid {
    fn from_reservation(reservation: Reservation, app_name: impl Into<String>) -> Self {
        let app_name = app_name.into();
        match reservation {
            Reservation::Keywords => Self::ReservedKeyword { app_name },
            Reservation::Windows => Self::ReservedWindows { app_name },
            Reservation::Artifacts => Self::ReservedArtifacts { app_name },
        }
    }

    pub fn suggested(&self) -> Option<&str> {
        match self {
            Invalid::NotAscii { suggested, .. } => suggested.as_ref(),
            Invalid::StartsWithDigit { suggested, .. } => suggested.as_ref(),
            Invalid::NotAlphanumericHyphenOrUnderscore { suggested, .. } => suggested.as_ref(),
            _ => None,
        }
        .map(|s| s.as_str())
    }
}

fn normalize_case(s: &str) -> String {
    if s.contains('_') {
        s.to_snek_case()
    } else {
        s.to_kebab_case()
    }
}

pub fn transliterate(s: &str) -> Option<String> {
    // `deunicode` guarantees that this will generate valid ASCII, so this would
    // guaranteed not to recurse even if we hadn't already split out a separate
    // non-recursive function.
    let all_ascii = normalize_case(&deunicode::deunicode(s));
    let transliterated = if has_initial_number(&all_ascii) {
        transliterate_initial_number(&all_ascii)
    } else {
        all_ascii
    };
    validate_non_recursive(transliterated).ok()
}

fn has_initial_number(s: &str) -> bool {
    s.chars().next().unwrap().is_ascii_digit()
}

fn transliterate_initial_number(s: &str) -> String {
    let (last_digit_indx, _) = s
        .char_indices()
        .take_while(|(_, c)| c.is_ascii_digit())
        .last()
        .expect("developer error: called `transliterate_initial_number` on an app name that didn't actually start with a number");
    let (number, tail) = s.split_at(last_digit_indx + 1);
    let number: i64 = number.parse().expect(
        "developer error: despite being digits, the initial digits couldn't be parsed as a number",
    );
    let transliterated = english_numbers::convert(
        number,
        english_numbers::Formatting {
            spaces: true,
            ..english_numbers::Formatting::none()
        },
    );
    normalize_case(&format!("{}-{}", transliterated, tail))
}

fn char_allowed(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-'
}

fn char_naughty(c: char) -> bool {
    !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '_' && c != '-'
}

fn strip_naughty_chars(s: &str) -> String {
    s.chars().filter(|c| char_allowed(*c)).collect()
}

fn validate_non_recursive<T: Deref<Target = str>>(app_name: T) -> Result<T, Invalid> {
    // crates.io and Android require alphanumeric ASCII with underscores
    // (crates.io also allows hyphens), which is stricter than Rust/Cargo's
    // general requirements, but being conservative here is a super good idea.
    // Rust also forbids some reserved keywords. We're extra aggressive and
    // forbid uppercase entirely, since it's very unconventional to use. We do
    // allow hyphens, so when hyphens are unacceptable `app_name_snake` must be
    // used.
    if !app_name.is_empty() {
        if app_name.is_ascii() {
            if has_initial_number(app_name.deref()) {
                Err(Invalid::StartsWithDigit {
                    app_name: app_name.to_owned(),
                    suggested: None,
                })
            } else {
                match is_reserved(app_name.deref()) {
                    Ok(()) => {
                        if app_name.chars().all(char_allowed) {
                            Ok(app_name)
                        } else {
                            let mut naughty_chars = Vec::new();
                            for c in app_name.chars().filter(|c| char_naughty(*c)) {
                                if !naughty_chars.contains(&c) {
                                    naughty_chars.push(c);
                                }
                            }
                            Err(Invalid::NotAlphanumericHyphenOrUnderscore {
                                app_name: app_name.to_owned(),
                                naughty_chars,
                                suggested: None,
                            })
                        }
                    }
                    Err(reservation) => {
                        Err(Invalid::from_reservation(reservation, app_name.deref()))
                    }
                }
            }
        } else {
            Err(Invalid::NotAscii {
                app_name: app_name.to_owned(),
                suggested: None,
            })
        }
    } else {
        Err(Invalid::Empty)
    }
}

pub fn validate<T: Deref<Target = str>>(app_name: T) -> Result<T, Invalid> {
    // Suggestion generation could recurse, so we have a separate slightly
    // dumber function that doesn't generate suggestions.
    let mut result = validate_non_recursive(app_name);
    if let Err(err) = result.as_mut() {
        assert!(err.suggested().is_none());
        match err {
            Invalid::NotAscii {
                app_name,
                suggested,
            } => {
                *suggested = transliterate(app_name.deref());
            }
            Invalid::StartsWithDigit {
                app_name,
                suggested,
            } => {
                *suggested = Some(transliterate_initial_number(app_name.deref()));
            }
            Invalid::NotAlphanumericHyphenOrUnderscore {
                app_name,
                suggested,
                ..
            } => {
                *suggested =
                    validate_non_recursive(strip_naughty_chars(&normalize_case(app_name))).ok();
            }
            _ => (),
        }
    }
    result
}
