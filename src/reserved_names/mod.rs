// Taken from: https://github.com/rust-lang/cargo/blob/5fe8ab57e2a88ccaaab0821c306203eb19edf8fd/src/cargo/util/restricted_names.rs

#![allow(dead_code)]

use thiserror::Error;

pub static KEYWORDS: &[&str] = &[
    "Self", "abstract", "as", "async", "await", "become", "box", "break", "const", "continue",
    "crate", "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "if", "impl",
    "in", "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref",
    "return", "self", "static", "struct", "super", "trait", "true", "try", "type", "typeof",
    "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

pub static WINDOWS: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

pub static ARTIFACTS: &[&str] = &["deps", "examples", "build", "incremental"];

pub fn in_keywords(s: impl AsRef<str>) -> bool {
    KEYWORDS.contains(&s.as_ref())
}

pub fn in_windows(s: impl AsRef<str>) -> bool {
    WINDOWS.contains(&s.as_ref())
}

pub fn in_artifacts(s: impl AsRef<str>) -> bool {
    ARTIFACTS.contains(&s.as_ref())
}

#[derive(Debug, Error)]
pub enum Reservation {
    #[error("identifier is a reserved keyword")]
    Keywords,
    #[error("identifier uses a reserved windows keyword")]
    Windows,
    #[error("identifier uses a reserved artifact")]
    Artifacts,
}

pub fn is_reserved(s: impl AsRef<str>) -> Result<(), Reservation> {
    let s = s.as_ref();
    if in_keywords(s) {
        Err(Reservation::Keywords)
    } else if in_windows(s) {
        Err(Reservation::Windows)
    } else if in_artifacts(s) {
        Err(Reservation::Artifacts)
    } else {
        Ok(())
    }
}

static PRIMITIVE_TYPES: &[&str] = &[
    "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize", "f32",
    "f64", "bool", "str",
];

pub fn in_primitive_types(s: impl AsRef<str>) -> bool {
    PRIMITIVE_TYPES.contains(&s.as_ref())
}
