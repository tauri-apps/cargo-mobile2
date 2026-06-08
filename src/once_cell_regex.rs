#[macro_export]
macro_rules! regex {
    ($re:expr) => {{
        use ::once_cell::sync::OnceCell;
        use ::regex::Regex;
        static RE: OnceCell<Regex> = OnceCell::new();
        RE.get_or_init(|| Regex::new($re).unwrap())
    }};
}

#[macro_export]
macro_rules! regex_multi_line {
    ($re:expr) => {{
        use ::once_cell::sync::OnceCell;
        use ::regex::Regex;
        static RE: OnceCell<Regex> = OnceCell::new();
        RE.get_or_init(|| {
            ::regex::RegexBuilder::new($re)
                .multi_line(true)
                .build()
                .unwrap()
        })
    }};
}

#[macro_export]
macro_rules! byte_regex {
    ($re:expr) => {{
        use ::once_cell::sync::OnceCell;
        use ::regex::bytes::Regex;
        static RE: OnceCell<Regex> = OnceCell::new();
        RE.get_or_init(|| Regex::new($re).unwrap())
    }};
}
