use regex::Regex;

#[macro_export]
macro_rules! regex {
    ($re:literal $(,)?) => {{
        static RE: $crate::exports::once_cell::sync::OnceCell<$crate::exports::regex::Regex> =
            $crate::exports::once_cell::sync::OnceCell::new();
        RE.get_or_init(|| $crate::exports::regex::Regex::new($re).unwrap())
    }};
}

#[macro_export]
macro_rules! regex_multi_line {
    ($re:literal $(,)?) => {{
        static RE: $crate::exports::once_cell::sync::OnceCell<$crate::exports::regex::Regex> =
            $crate::exports::once_cell::sync::OnceCell::new();
        RE.get_or_init(|| {
            $crate::exports::regex::RegexBuilder::new($re)
                .multi_line(true)
                .build()
                .unwrap()
        })
    }};
}

pub fn has_match(re: &Regex, body: &str, pattern: &str) -> bool {
    re.captures(body)
        .and_then(|caps| {
            caps.iter()
                .find(|cap| cap.map(|cap| cap.as_str() == pattern).unwrap_or_default())
        })
        .is_some()
}
