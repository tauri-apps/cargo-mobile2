#![deny(unsafe_code)]

pub mod android;
#[cfg(target_os = "macos")]
pub mod apple;
pub mod bicycle;
pub mod config;
pub mod device;
pub mod doctor;
pub mod dot_cargo;
pub mod env;
pub mod init;
mod once_cell_regex;
pub mod open_harmony;
pub mod opts;
pub mod os;
mod project;
pub mod reserved_names;
pub mod target;
mod templating;
pub mod update;
pub mod util;
use std::ffi::OsStr;

pub use duct::Handle as ChildHandle;

pub static NAME: &str = "mobile";

trait DuctExpressionExt {
    fn vars(self, vars: impl IntoIterator<Item = (impl AsRef<OsStr>, impl AsRef<OsStr>)>) -> Self;
    fn run_and_detach(&self) -> Result<(), std::io::Error>;
    // Sets the stdin, stdout and stderr to properly
    // show the command output in a Node.js wrapper (napi-rs).
    fn dup_stdio(&self) -> Self;
    // Renders the program and arguments for error messages. The `Debug` impl
    // also dumps every env var and redirection wrapping the command.
    fn display_cmd(&self) -> String;
    // Like `display_cmd`, with extra arguments appended. Use it for arguments
    // added in a `before_spawn` hook, which the `Debug` output does not show.
    fn display_cmd_with<I, S>(&self, args: I) -> String
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>;
}

// Quotes an argument for display when it would otherwise be ambiguous.
// Backslashes are kept as-is so Windows paths stay copy-pasteable.
fn display_arg(arg: &str) -> String {
    if !arg.is_empty() && !arg.contains(|c: char| c.is_whitespace() || c == '"' || c == '\'') {
        return arg.to_string();
    }
    format!("\"{}\"", arg.replace('"', "\\\""))
}

// Reads a Rust `Debug` string literal (the opening quote is already consumed)
// and returns its decoded contents. Bytes that are not valid UTF-8 (printed as
// `\xHH` by `OsStr`'s `Debug`) are replaced with U+FFFD.
fn read_debug_str(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut out = String::new();
    while let Some(c) = chars.next() {
        match c {
            '"' => break,
            '\\' => match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('0') => out.push('\0'),
                Some('x') => {
                    chars.next();
                    chars.next();
                    out.push(char::REPLACEMENT_CHARACTER);
                }
                Some('u') => {
                    let mut hex = String::new();
                    if chars.peek() == Some(&'{') {
                        chars.next();
                        for c in chars.by_ref() {
                            if c == '}' {
                                break;
                            }
                            hex.push(c);
                        }
                    }
                    out.push(
                        u32::from_str_radix(&hex, 16)
                            .ok()
                            .and_then(char::from_u32)
                            .unwrap_or(char::REPLACEMENT_CHARACTER),
                    );
                }
                Some(c) => out.push(c),
                None => break,
            },
            c => out.push(c),
        }
    }
    out
}

// Extracts each `Cmd([...])` node from a duct expression's `Debug` output
// and joins its arguments, so pipes render as `a | b`. String literals are
// skipped while searching, so env values or paths that happen to contain
// `Cmd([` are never mistaken for a command. Returns `None` if no command is
// found (e.g. duct changed its `Debug` format).
fn display_expression_debug(debug: &str) -> Option<String> {
    const MARKER: &str = "Cmd([";
    let mut commands = Vec::new();
    let mut chars = debug.chars().peekable();
    let mut token = String::new();
    while let Some(c) = chars.next() {
        if c == '"' {
            read_debug_str(&mut chars);
            token.clear();
            continue;
        }
        token.push(c);
        if !MARKER.starts_with(token.as_str()) {
            token.clear();
            if MARKER.starts_with(c) {
                token.push(c);
            }
            continue;
        }
        if token != MARKER {
            continue;
        }
        token.clear();
        let mut args = Vec::new();
        while let Some(c) = chars.next() {
            match c {
                '"' => args.push(display_arg(&read_debug_str(&mut chars))),
                ']' => break,
                _ => {}
            }
        }
        commands.push(args.join(" "));
    }
    (!commands.is_empty()).then(|| commands.join(" | "))
}

impl DuctExpressionExt for duct::Expression {
    fn vars(
        mut self,
        vars: impl IntoIterator<Item = (impl AsRef<OsStr>, impl AsRef<OsStr>)>,
    ) -> Self {
        for (k, v) in vars {
            self = self.env(&k, &v);
        }
        self
    }

    fn run_and_detach(&self) -> Result<(), std::io::Error> {
        self.before_spawn(|cmd| {
            // This is pretty much lifted from the implementation in Alacritty:
            // https://github.com/alacritty/alacritty/blob/8bd2c13490f8cb6ad6b0c1104f9586b3554efea2/alacritty/src/daemon.rs
            #[cfg(unix)]
            #[allow(unsafe_code)]
            unsafe {
                use std::os::unix::process::CommandExt as _;

                let display = format!("{cmd:?}");
                cmd.pre_exec(move || match libc::fork() {
                    -1 => {
                        let err = std::io::Error::last_os_error();
                        log::error!("`fork` failed for command {:?}: {}", display, err);
                        Err(err)
                    }
                    0 => {
                        if libc::setsid() == -1 {
                            let err = std::io::Error::last_os_error();
                            log::error!("`setsid` failed for command {:?}: {}", display, err);
                            Err(err)
                        } else {
                            Ok(())
                        }
                    }
                    _ => libc::_exit(0),
                });
            }
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
                const CREATE_NO_WINDOW: u32 = 0x08000000;
                cmd.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
            }

            Ok(())
        })
        .stdin_null()
        .stdout_null()
        .stderr_null()
        .start()?;
        Ok(())
    }

    fn dup_stdio(&self) -> Self {
        self.stdin_file(os_pipe::dup_stdin().unwrap())
            .stdout_file(os_pipe::dup_stdout().unwrap())
            .stderr_file(os_pipe::dup_stderr().unwrap())
    }

    fn display_cmd(&self) -> String {
        let debug = format!("{self:?}");
        display_expression_debug(&debug).unwrap_or(debug)
    }

    fn display_cmd_with<I, S>(&self, args: I) -> String
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut out = self.display_cmd();
        for arg in args {
            out.push(' ');
            out.push_str(&display_arg(&arg.as_ref().to_string_lossy()));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::DuctExpressionExt as _;

    #[test]
    fn display_cmd_omits_env_and_redirections() {
        let cmd = duct::cmd("adb", ["-s", "emulator-5554", "path with space"])
            .env("SECRET", "value")
            .stdout_null()
            .before_spawn(|_| Ok(()));
        assert_eq!(
            cmd.display_cmd(),
            r#"adb -s emulator-5554 "path with space""#
        );
    }

    #[test]
    fn display_cmd_renders_pipes() {
        let cmd = duct::cmd!("adb", "logcat")
            .env("A", "B")
            .pipe(duct::cmd!("ndk-stack"));
        assert_eq!(cmd.display_cmd(), "adb logcat | ndk-stack");
    }

    #[test]
    fn display_cmd_ignores_marker_inside_strings() {
        let cmd = duct::cmd("prog", ["arg"])
            .env("X", r#"foo Cmd(["evil"]) bar"#)
            .dir(r#"/tmp/Cmd(["d"])"#);
        assert_eq!(cmd.display_cmd(), "prog arg");
    }

    #[test]
    fn display_cmd_decodes_escapes() {
        let cmd = duct::cmd("prog", ["a\tb", "\u{e9}\u{1b}", "nl\n"]);
        assert_eq!(cmd.display_cmd(), "prog \"a\tb\" \u{e9}\u{1b} \"nl\n\"");
    }

    #[test]
    fn display_cmd_keeps_backslashes_and_quotes_quotes() {
        let cmd = duct::cmd("prog", [r"C:\Program Files\x", r"C:\sdk", r#"x"y"#, ""]);
        assert_eq!(
            cmd.display_cmd(),
            r#"prog "C:\Program Files\x" C:\sdk "x\"y" """#
        );
    }

    #[test]
    fn display_cmd_with_quotes_extra_args() {
        let cmd = duct::cmd!("adb").before_spawn(|_| Ok(()));
        assert_eq!(
            cmd.display_cmd_with(["install", "/a b/app.apk"]),
            r#"adb install "/a b/app.apk""#
        );
    }

    #[test]
    fn display_expression_debug_returns_none_without_cmd() {
        assert_eq!(super::display_expression_debug(r#"Other(["a"])"#), None);
    }
}
