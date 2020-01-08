use crate::util;
use heck::{KebabCase as _, SnekCase as _};
use std::{fmt, ops::Deref};

// https://github.com/rust-lang/cargo/blob/57986eac7157261c33f0123bade7ccd20f15200f/src/cargo/ops/cargo_new.rs#L141
// https://doc.rust-lang.org/grammar.html#keywords
static BLACKLIST: &'static [&'static str] = &[
    "abstract", "alignof", "as", "become", "box", "break", "const", "continue", "crate", "do",
    "else", "enum", "extern", "false", "final", "fn", "for", "if", "impl", "in", "let", "loop",
    "macro", "match", "mod", "move", "mut", "offsetof", "override", "priv", "proc", "pub", "pure",
    "raw", "ref", "return", "self", "sizeof", "static", "struct", "super", "test", "trait", "true",
    "type", "typeof", "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

#[derive(Debug)]
pub enum Invalid {
    Empty,
    NotAscii {
        app_name: String,
        suggested: Option<String>,
    },
    Blacklisted {
        app_name: String,
    },
    NotAlphanumericHyphenOrUnderscore {
        app_name: String,
        naughty_chars: Vec<char>,
        suggested: Option<String>,
    },
}

impl fmt::Display for Invalid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Invalid::Empty => write!(f, "The app name can't be empty.")?,
            Invalid::NotAscii { app_name, .. } => write!(
                f,
                "\"{}\" isn't valid ASCII.",
                app_name,
            )?,
            Invalid::Blacklisted { app_name } => write!(
                f,
                "\"{}\" is a reserved keyword: https://doc.rust-lang.org/grammar.html#keywords",
                app_name,
            )?,
            Invalid::NotAlphanumericHyphenOrUnderscore { app_name, naughty_chars, .. } => write!(
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
    pub fn suggested(&self) -> Option<&str> {
        match self {
            Invalid::NotAscii { suggested, .. } => suggested.as_ref(),
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
    validate_non_recursive(normalize_case(&deunicode::deunicode(s))).ok()
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
            if !BLACKLIST.contains(&app_name.deref()) {
                if app_name.chars().all(|c| char_allowed(c)) {
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
            } else {
                Err(Invalid::Blacklisted {
                    app_name: app_name.to_owned(),
                })
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
    match result.as_mut() {
        Err(Invalid::NotAlphanumericHyphenOrUnderscore {
            app_name,
            suggested,
            ..
        }) => {
            assert!(suggested.is_none());
            *suggested =
                validate_non_recursive(strip_naughty_chars(&normalize_case(&app_name))).ok();
        }
        Err(Invalid::NotAscii {
            app_name,
            suggested,
        }) => {
            assert!(suggested.is_none());
            *suggested = transliterate(&app_name.deref());
        }
        _ => (),
    }
    result
}
