use std::{
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};

use super::adb;
use crate::regex;
use crate::{
    android::env::Env,
    util::cli::{Report, Reportable},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to run `adb emu avd name`: {0}")]
    EmuFailed(#[source] super::RunCheckedError),
    #[error(transparent)]
    GetPropFailed(super::get_prop::Error),
    #[error("Failed to run `adb shell dumpsys bluetooth_manager`: {0}")]
    DumpsysFailed(#[source] super::RunCheckedError),
    #[error("Name regex didn't match anything.")]
    NotMatched,
    #[error("Failed to run {command}: {error}")]
    CommandFailed {
        command: String,
        error: std::io::Error,
    },
}

impl Reportable for Error {
    fn report(&self) -> Report {
        let msg = "Failed to get device name";
        match self {
            Self::EmuFailed(err) => err.report("Failed to run `adb emu avd name`"),
            Self::GetPropFailed(err) => err.report(),
            Self::DumpsysFailed(err) => {
                err.report("Failed to run `adb shell dumpsys bluetooth_manager`")
            }
            Self::NotMatched => Report::error(msg, self),
            Self::CommandFailed { command, error } => {
                Report::error(format!("Failed to run {command}"), error)
            }
        }
    }
}

pub fn device_name(env: &Env, serial_no: &str) -> Result<String, Error> {
    if serial_no.starts_with("emulator") {
        let cmd = adb(env, ["-s", serial_no, "emu", "avd", "name"])
            .stderr_capture()
            .stdout_capture();
        let name = super::check_authorized(
            cmd.start()
                .map_err(|error| Error::CommandFailed {
                    command: format!("{cmd:?}"),
                    error,
                })?
                .wait_timeout(Duration::from_secs(3))
                .and_then(|output| {
                    output.ok_or(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "`adb emu avd name` timed out",
                    ))
                })
                .map_err(|error| Error::CommandFailed {
                    command: format!("{cmd:?}"),
                    error,
                })?,
        )
        .map(|stdout| stdout.split('\n').next().unwrap().trim().to_string())
        .map_err(Error::EmuFailed)?;
        if name.is_empty() {
            if let Some(port) = serial_no
                .strip_prefix("emulator-")
                .and_then(|port_str| port_str.parse::<u16>().ok())
            {
                if let Some(name) = device_name_from_emulator_console(port) {
                    return Ok(name);
                }
            }
            super::get_prop::get_prop(env, serial_no, "ro.boot.qemu.avd_name")
                .map_err(Error::GetPropFailed)
        } else {
            Ok(name)
        }
    } else {
        let cmd = adb(
            env,
            ["-s", serial_no, "shell", "dumpsys", "bluetooth_manager"],
        )
        .stderr_capture()
        .stdout_capture();
        super::check_authorized(
            cmd.start()
                .map_err(|error| Error::CommandFailed {
                    command: format!("{cmd:?}"),
                    error,
                })?
                .wait_timeout(Duration::from_secs(3))
                .and_then(|output| {
                    output.ok_or(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "`adb shell dumpsys bluetooth_manager` timed out",
                    ))
                })
                .map_err(|error| Error::CommandFailed {
                    command: format!("{cmd:?}"),
                    error,
                })?,
        )
        .map_err(Error::DumpsysFailed)
        .and_then(|stdout| {
            regex!(r"\bname: (?P<name>.*)")
                .captures(&stdout)
                .map(|caps| caps["name"].to_owned())
                .ok_or(Error::NotMatched)
        })
    }
}

fn device_name_from_emulator_console(port: u16) -> Option<String> {
    let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port)) else {
        return None;
    };

    // short timeout so it doesn't hang
    let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));

    let _ = stream.write_all(b"avd name\n");
    let mut buf = String::new();
    let _ = stream.read_to_string(&mut buf);

    // filter out the "OK" and "Android Console" lines
    let name = buf
        .lines()
        .filter(|line| !line.contains("OK") && !line.contains("Android Console"))
        .collect::<Vec<_>>()
        .join("")
        .trim()
        .to_string();

    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}
