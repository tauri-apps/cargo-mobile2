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
}

// Extracts each `Cmd([...])` node from a duct expression's `Debug` output
// and joins its arguments, so pipes render as `a | b`.
fn display_expression_debug(debug: &str) -> String {
    let mut commands = Vec::new();
    let mut rest = debug;
    while let Some(start) = rest.find("Cmd([") {
        rest = &rest[start + "Cmd([".len()..];
        let mut args = Vec::new();
        let mut chars = rest.char_indices();
        let mut end = rest.len();
        while let Some((i, c)) = chars.next() {
            match c {
                '"' => {
                    let mut arg = String::new();
                    while let Some((_, c)) = chars.next() {
                        match c {
                            '\\' => {
                                if let Some((_, escaped)) = chars.next() {
                                    arg.push(escaped);
                                }
                            }
                            '"' => break,
                            c => arg.push(c),
                        }
                    }
                    if arg.is_empty() || arg.contains(char::is_whitespace) {
                        args.push(format!("{arg:?}"));
                    } else {
                        args.push(arg);
                    }
                }
                ']' => {
                    end = i;
                    break;
                }
                _ => {}
            }
        }
        rest = &rest[end..];
        commands.push(args.join(" "));
    }
    commands.join(" | ")
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
        display_expression_debug(&format!("{self:?}"))
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
}
