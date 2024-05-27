use crate::util::list_display;
use std::error::Error;
use std::fmt;

static RESERVED_PACKAGE_NAMES: [&str; 2] = ["kotlin", "java"];
// https://docs.oracle.com/javase/tutorial/java/nutsandbolts/_keywords.html
static RESERVED_JAVA_KEYWORDS: [&str; 53] = [
    "abstract",
    "assert",
    "boolean",
    "break",
    "byte",
    "case",
    "catch",
    "char",
    "class",
    "const",
    "continue",
    "default",
    "do",
    "double",
    "else",
    "enum",
    "extends",
    "false",
    "final",
    "finally",
    "float",
    "for",
    "goto",
    "if",
    "implements",
    "import",
    "instanceof",
    "int",
    "interface",
    "long",
    "native",
    "new",
    "null",
    "package",
    "private",
    "protected",
    "public",
    "return",
    "short",
    "static",
    "strictfp",
    "super",
    "switch",
    "synchronized",
    "this",
    "throw",
    "throws",
    "transient",
    "true",
    "try",
    "void",
    "volatile",
    "while",
];

#[derive(Debug)]
pub enum IdentifierError {
    Empty,
    NotAsciiAlphanumeric { bad_chars: Vec<char> },
    StartsWithDigit { label: String },
    ReservedPackageName { package_name: String },
    ReservedKeyword { keyword: String },
    StartsOrEndsWithADot,
    EmptyLabel,
}

impl Error for IdentifierError {}

impl fmt::Display for IdentifierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "Identifier can't be empty."),
            Self::NotAsciiAlphanumeric { bad_chars } => write!(
                f,
                "{} characters were used in identifier, but only ASCII letters and numbers are allowed.",
                list_display(
                    &bad_chars
                        .iter()
                        .map(|c| format!("'{}'", c))
                        .collect::<Vec<_>>()
                ),
            ),
            Self::ReservedPackageName { package_name } => write!(
                f,
                "\"{}\" is a reserved package name in this project and can't be used as a top-level identifier.",
                package_name
            ),
            Self::ReservedKeyword { keyword } => write!(
                f,
                "\"{}\" is a reserved keyword in java/kotlin and can't be used. For more info, please visit https://kotlinlang.org/docs/reference/keyword-reference.html and https://docs.oracle.com/javase/tutorial/java/nutsandbolts/_keywords.html",
                keyword
            ),
            Self::StartsWithDigit { label } => write!(
                f,
                "\"{}\" label starts with a digit, which is not allowed in java/kotlin packages.",
                label
            ),
            Self::StartsOrEndsWithADot => write!(f, "Identifier can't start or end with a dot."),
            Self::EmptyLabel => write!(f, "Labels can't be empty."),
        }
    }
}

pub fn check_identifier_syntax(identifier_name: &str) -> Result<(), IdentifierError> {
    if identifier_name.is_empty() {
        return Err(IdentifierError::Empty);
    }
    if identifier_name.starts_with('.') || identifier_name.ends_with('.') {
        return Err(IdentifierError::StartsOrEndsWithADot);
    }
    let labels = identifier_name.split('.');
    for label in labels {
        if label.is_empty() {
            return Err(IdentifierError::EmptyLabel);
        }
        if RESERVED_JAVA_KEYWORDS.contains(&label) {
            return Err(IdentifierError::ReservedKeyword {
                keyword: label.to_owned(),
            });
        }
        if label.chars().next().unwrap().is_ascii_digit() {
            return Err(IdentifierError::StartsWithDigit {
                label: label.to_owned(),
            });
        }
        let mut bad_chars = Vec::new();
        for c in label.chars() {
            if !c.is_ascii_alphanumeric() && !bad_chars.contains(&c) {
                bad_chars.push(c);
            }
        }
        if !bad_chars.is_empty() {
            return Err(IdentifierError::NotAsciiAlphanumeric { bad_chars });
        }
    }
    for pkg_name in RESERVED_PACKAGE_NAMES.iter() {
        if identifier_name.ends_with(pkg_name) {
            return Err(IdentifierError::ReservedPackageName {
                package_name: pkg_name.to_string(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use rstest::rstest;

    #[rstest(
        input,
        case("com.example"),
        case("t2900.e1.s709.t1000"),
        case("kotlin.com"),
        case("java.test"),
        case("synchronized2.com")
    )]
    fn test_check_identifier_syntax_correct(input: &str) {
        check_identifier_syntax(input).unwrap();
    }

    #[rstest(input, error,
        case("ラスト.テスト", IdentifierError::NotAsciiAlphanumeric { bad_chars: vec!['ラ', 'ス', 'ト'] }),
        case("test.digits.87", IdentifierError::StartsWithDigit { label: String::from("87") }),
        case("", IdentifierError::Empty {}),
        case(".bad.dot.syntax", IdentifierError::StartsOrEndsWithADot {}),
        case("com.kotlin", IdentifierError::ReservedPackageName { package_name: String::from("kotlin") }),
        case("some.identifier.catch.com", IdentifierError::ReservedKeyword { keyword: String::from("catch") }),
        case("com..empty.label", IdentifierError::EmptyLabel)
    )]
    fn test_check_identifier_syntax_error(input: &str, error: IdentifierError) {
        assert_eq!(
            check_identifier_syntax(input).unwrap_err().to_string(),
            error.to_string()
        )
    }
}
