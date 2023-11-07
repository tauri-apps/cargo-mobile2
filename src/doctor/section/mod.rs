pub mod android;
#[cfg(target_os = "macos")]
pub mod apple;
pub mod cargo_mobile;
pub mod device_list;

use crate::util::{
    self,
    cli::{colors, TextWrapper},
};
use colored::Colorize as _;
use std::fmt::Debug;

#[derive(Clone, Copy, Debug)]
enum Label {
    Victory,
    Warning,
    Error,
}

impl Label {
    fn title_symbol(self) -> &'static str {
        match self {
            Self::Victory | Self::Warning => "✔",
            Self::Error => "!",
        }
    }

    fn item_symbol(self) -> &'static str {
        match self {
            Self::Victory => "•",
            Self::Warning | Self::Error => "✗",
        }
    }

    fn color(self) -> colored::Color {
        match self {
            Self::Victory => colors::VICTORY,
            Self::Warning => colors::WARNING,
            Self::Error => colors::ERROR,
        }
    }

    fn format_title(self, title: &str) -> colored::ColoredString {
        format!("[{}] {}", self.title_symbol(), title)
            .color(self.color())
            .bold()
    }

    fn format_item(self, msg: &str) -> colored::ColoredString {
        let item = format!("{} {}", self.item_symbol(), msg);
        match self {
            Self::Victory => item.normal(),
            _ => item.color(self.color()).bold(),
        }
    }
}

#[derive(Debug)]
struct Item {
    label: Label,
    msg: String,
}

impl<T: ToString, E: ToString> From<Result<T, E>> for Item {
    fn from(result: Result<T, E>) -> Item {
        Self::from_result(result)
    }
}

impl Item {
    fn new(label: Label, msg: impl ToString) -> Self {
        Self {
            label,
            msg: msg.to_string(),
        }
    }

    fn victory(msg: impl ToString) -> Self {
        Self::new(Label::Victory, msg)
    }

    #[cfg(target_os = "macos")]
    fn warning(msg: impl ToString) -> Self {
        Self::new(Label::Warning, msg)
    }

    fn failure(msg: impl ToString) -> Self {
        Self::new(Label::Error, msg)
    }

    fn from_result(result: Result<impl ToString, impl ToString>) -> Self {
        util::unwrap_either(result.map(Self::victory).map_err(Self::failure))
    }

    fn is_warning(&self) -> bool {
        matches!(self.label, Label::Warning)
    }

    fn is_failure(&self) -> bool {
        matches!(self.label, Label::Error)
    }

    fn format(&self) -> colored::ColoredString {
        self.label.format_item(&self.msg)
    }
}

#[derive(Debug)]
pub struct Section {
    title: String,
    items: Vec<Item>,
}

impl Section {
    fn new(title: impl ToString) -> Self {
        Self {
            title: title.to_string(),
            items: Default::default(),
        }
    }

    fn with_item(mut self, item: impl Into<Item>) -> Self {
        self.items.push(item.into());
        self
    }

    fn with_victory(self, victory: impl ToString) -> Self {
        self.with_item(Item::victory(victory))
    }

    fn with_failure(self, failure: impl ToString) -> Self {
        self.with_item(Item::failure(failure))
    }

    fn with_items(mut self, items: impl IntoIterator<Item = impl Into<Item>>) -> Self {
        self.items.extend(items.into_iter().map(Into::into));
        self
    }

    fn with_victories(self, victories: impl IntoIterator<Item = impl ToString>) -> Self {
        self.with_items(victories.into_iter().map(Item::victory))
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn has_error(&self) -> bool {
        self.items.iter().any(Item::is_failure)
    }

    fn has_warning(&self) -> bool {
        self.items.iter().any(Item::is_warning)
    }

    fn label(&self) -> Label {
        if self.has_error() {
            Label::Error
        } else if self.has_warning() {
            Label::Warning
        } else {
            Label::Victory
        }
    }

    pub fn print(&self, wrapper: &TextWrapper) {
        static BULLET_INDENT: &str = "    ";
        static HANGING_INDENT: &str = "      ";
        let bullet_wrapper = TextWrapper(
            wrapper
                .clone()
                .0
                .initial_indent(BULLET_INDENT)
                .subsequent_indent(HANGING_INDENT),
        );
        println!(
            "\n{}",
            // The `.to_string()` at the end is necessary for the color/bold to
            // actually show - otherwise, the colored string just `AsRef`s to
            // satisfy `TextWrapper::fill` and the formatting is left behind.
            wrapper.fill(&self.label().format_title(&self.title))
        );
        for report_bullet in &self.items {
            println!("{}", bullet_wrapper.fill(&report_bullet.format()));
        }
    }
}
