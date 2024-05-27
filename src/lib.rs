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
    fn run_and_detach(self) -> Result<(), std::io::Error>;
    // Sets the stdin, stdout and stderr to properly
    // show the command output in a Node.js wrapper (napi-rs).
    fn dup_stdio(&self) -> Self;
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

    fn run_and_detach(self) -> Result<(), std::io::Error> {
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
}
