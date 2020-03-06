pub mod device_list;
pub mod device_name;
pub mod get_prop;
pub mod service_call;

pub use self::{
    device_list::device_list, device_name::device_name, get_prop::get_prop,
    service_call::service_call,
};

use crate::env::Env;
use ginit_core::util::PureCommand;
use std::process::Command;

pub fn adb(env: &Env, serial_no: &str) -> Command {
    let mut command = PureCommand::new("adb", env);
    command.args(&["-s", serial_no]);
    command
}
