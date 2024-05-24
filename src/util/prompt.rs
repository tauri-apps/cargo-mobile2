use colored::{Color, Colorize as _};
use std::{
    fmt::Display,
    io::{self, Write},
};

pub fn minimal(msg: impl Display) -> io::Result<String> {
    let mut input = String::new();
    print!("{}: ", msg);
    io::stdout().flush()?;
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_owned())
}

pub fn default(
    msg: impl Display,
    default: Option<&str>,
    default_color: Option<Color>,
) -> io::Result<String> {
    if let Some(default) = default {
        let msg = if let Some(default_color) = default_color {
            format!("{} ({})", msg, default.color(default_color).bold())
        } else {
            format!("{} ({})", msg, default)
        };
        minimal(msg)
    } else {
        minimal(msg)
    }
    .map(|response| {
        #[allow(clippy::unnecessary_unwrap)]
        if response.is_empty() && default.is_some() {
            default.unwrap().to_owned()
        } else {
            response
        }
    })
}

pub fn yes_no(msg: impl Display, default: Option<bool>) -> io::Result<Option<bool>> {
    let y_n = match default {
        Some(true) => "[Y/n]",
        Some(false) => "[y/N]",
        None => "[y/n]",
    };
    minimal(format!("{} {}", msg, y_n)).map(|response| {
        if response.eq_ignore_ascii_case("y") {
            Some(true)
        } else if response.eq_ignore_ascii_case("n") {
            Some(false)
        } else if response.is_empty() {
            default
        } else {
            println!("That was neither a Y nor an N! You're pretty silly.");
            None
        }
    })
}

pub fn list_display_only(choices: impl Iterator<Item = impl Display>, choice_count: usize) {
    if choice_count > 0 {
        for (index, choice) in choices.enumerate() {
            println!("  [{}] {}", index.to_string().green(), choice);
        }
    } else {
        println!("  -- none --");
    }
}

pub fn list(
    header: impl Display,
    choices: impl ExactSizeIterator<Item = impl Display>,
    noun: impl Display,
    alternative: Option<&str>,
    msg: impl Display,
) -> io::Result<usize> {
    println!("{}:", header);
    let choice_count = choices.len();
    list_display_only(choices, choice_count);
    if let Some(alternative) = alternative {
        println!(
            "  Enter an {} for a {} above, or enter a {} manually.",
            "index".green(),
            noun,
            alternative.cyan(),
        );
    } else {
        println!("  Enter an {} for a {} above.", "index".green(), noun);
    }
    loop {
        let response = default(
            &msg,
            if choice_count == 1 { Some("0") } else { None },
            Some(Color::Green),
        )?;
        if !response.is_empty() {
            if let Ok(index) = response.parse::<usize>() {
                if index < choice_count {
                    return Ok(index);
                } else {
                    println!("There's no device with an index that high.");
                }
            } else {
                println!("Hey, that wasn't a number! You're silly.");
            }
        } else {
            println!("Not to be pushy, but you need to pick a device.");
        }
    }
}
