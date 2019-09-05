use colored::*;
use std::{
    fmt::Display,
    io::{self, Write},
};

pub fn minimal(msg: impl Display) -> io::Result<String> {
    let mut input = String::new();
    print!("{}: ", msg);
    io::stdout().flush()?;
    io::stdin().read_line(&mut input)?;
    input = input.trim().to_owned();
    Ok(input)
}

pub fn default(
    msg: impl Display,
    default: Option<&str>,
    default_color: Option<Color>,
) -> io::Result<String> {
    if let Some(default) = default {
        let msg = if let Some(default_color) = default_color {
            format!("{} ({})", msg, default.color(default_color))
        } else {
            format!("{} ({})", msg, default)
        };
        minimal(msg)
    } else {
        minimal(msg)
    }
    .map(|response| {
        if response.is_empty() && default.is_some() {
            default.unwrap().to_owned()
        } else {
            response
        }
    })
}

#[derive(Clone, Copy, Debug)]
pub enum YesOrNo {
    Yes,
    No,
}

pub fn yes_no(msg: impl Display, default: Option<YesOrNo>) -> io::Result<Option<YesOrNo>> {
    let y_n = match default {
        Some(YesOrNo::Yes) => "[Y/n]",
        Some(YesOrNo::No) => "[y/N]",
        None => "[y/n]",
    };
    minimal(&format!("{} {}", msg, y_n)).map(|response| {
        if response.eq_ignore_ascii_case("y") {
            Some(YesOrNo::Yes)
        } else if response.eq_ignore_ascii_case("n") {
            Some(YesOrNo::No)
        } else if response.is_empty() {
            default
        } else {
            println!("That was neither a Y nor an N! You're pretty silly.");
            None
        }
    })
}
