use crate::exports::regex::Regex;

pub fn has_match(re: &Regex, body: &str, pattern: &str) -> bool {
    re.captures(body)
        .and_then(|caps| {
            caps.iter()
                .find(|cap| cap.map(|cap| cap.as_str() == pattern).unwrap_or_default())
        })
        .is_some()
}
