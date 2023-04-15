use crate::util::list_display;
use std::error::Error;
use std::fmt;

static RESERVED_PACKAGE_NAMES: [&str; 2] = ["kotlin", "java"];
static RESERVED_KEYWORDS: [&str; 63] = [
    "abstract",
    "as",
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
    "fun",
    "goto",
    "if",
    "implements",
    "import",
    "instanceof",
    "in",
    "int",
    "interface",
    "is",
    "long",
    "native",
    "new",
    "null",
    "object",
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
    "typealias",
    "typeof",
    "val",
    "var",
    "void",
    "volatile",
    "when",
    "while",
];

#[derive(Debug)]
pub enum DomainError {
    Empty,
    NotAsciiAlphanumeric { bad_chars: Vec<char> },
    StartsWithDigit { label: String },
    ReservedPackageName { package_name: String },
    ReservedKeyword { keyword: String },
    StartsOrEndsWithADot,
    EmptyLabel,
}

impl Error for DomainError {}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "Domain can't be empty."),
            Self::NotAsciiAlphanumeric { bad_chars } => write!(
                f,
                "{} characters were used in domain, but only ASCII letters and numbers are allowed.",
                list_display(
                    &bad_chars
                        .iter()
                        .map(|c| format!("'{}'", c))
                        .collect::<Vec<_>>()
                ),
            ),
            Self::ReservedPackageName { package_name } => write!(
                f,
                "\"{}\" is a reserved package name in this project and can't be used as a top-level domain.",
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
            Self::StartsOrEndsWithADot => write!(f, "Domain can't start or end with a dot."),
            Self::EmptyLabel => write!(f, "Labels can't be empty."),
        }
    }
}

pub fn check_domain_syntax(domain_name: &str) -> Result<(), DomainError> {
    if domain_name.is_empty() {
        return Err(DomainError::Empty);
    }
    if domain_name.starts_with('.') || domain_name.ends_with('.') {
        return Err(DomainError::StartsOrEndsWithADot);
    }
    let labels = domain_name.split('.');
    for label in labels {
        if label.is_empty() {
            return Err(DomainError::EmptyLabel);
        }
        if RESERVED_KEYWORDS.contains(&label) {
            return Err(DomainError::ReservedKeyword {
                keyword: label.to_owned(),
            });
        }
        if label.chars().next().unwrap().is_ascii_digit() {
            return Err(DomainError::StartsWithDigit {
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
            return Err(DomainError::NotAsciiAlphanumeric { bad_chars });
        }
    }
    for pkg_name in RESERVED_PACKAGE_NAMES.iter() {
        if domain_name.ends_with(pkg_name) {
            return Err(DomainError::ReservedPackageName {
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
    fn test_check_domain_syntax_correct(input: &str) {
        check_domain_syntax(input).unwrap();
    }

    #[rstest(input, error,
        case("ラスト.テスト", DomainError::NotAsciiAlphanumeric { bad_chars: vec!['ラ', 'ス', 'ト'] }),
        case("test.digits.87", DomainError::StartsWithDigit { label: String::from("87") }),
        case("", DomainError::Empty {}),
        case(".bad.dot.syntax", DomainError::StartsOrEndsWithADot {}),
        case("com.kotlin", DomainError::ReservedPackageName { package_name: String::from("kotlin") }),
        case("some.domain.catch.com", DomainError::ReservedKeyword { keyword: String::from("catch") }),
        case("com..empty.label", DomainError::EmptyLabel)
    )]
    fn test_check_domain_syntax_error(input: &str, error: DomainError) {
        assert_eq!(
            check_domain_syntax(input).unwrap_err().to_string(),
            error.to_string()
        )
    }
}
