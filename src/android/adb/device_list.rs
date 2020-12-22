use super::{device_name, get_prop};
use crate::{
    android::{device::Device, env::Env, target::Target},
    env::ExplicitEnv as _,
    util::cli::{Report, Reportable},
};
use once_cell_regex::regex_multi_line;
use std::collections::BTreeSet;

#[derive(Debug)]
pub enum Error {
    DevicesFailed(super::RunCheckedError),
    NameFailed(device_name::Error),
    ModelFailed(get_prop::Error),
    AbiFailed(get_prop::Error),
    AbiInvalid(String),
}

impl Reportable for Error {
    fn report(&self) -> Report {
        let msg = "Failed to detect connected Android devices";
        match self {
            Self::DevicesFailed(err) => err.report("Failed to run `adb devices`"),
            Self::NameFailed(err) => err.report(),
            Self::ModelFailed(err) | Self::AbiFailed(err) => err.report(),
            Self::AbiInvalid(abi) => {
                Report::error(msg, format!("{:?} isn't a valid target ABI.", abi))
            }
        }
    }
}

pub fn device_list(env: &Env) -> Result<BTreeSet<Device<'static>>, Error> {
    super::check_authorized(
        bossy::Command::pure("adb")
            .with_env_vars(env.explicit_env())
            .with_args(&["devices"])
            .run_and_wait_for_str(|raw_list| {
                regex_multi_line!(r"^([\w\d]{6,20})	\b")
                    .captures_iter(raw_list)
                    .map(|caps| {
                        assert_eq!(caps.len(), 2);
                        let serial_no = caps.get(1).unwrap().as_str().to_owned();
                        let name = device_name(env, &serial_no).map_err(Error::NameFailed)?;
                        let model = get_prop(env, &serial_no, "ro.product.model")
                            .map_err(Error::ModelFailed)?;
                        let abi = get_prop(env, &serial_no, "ro.product.cpu.abi")
                            .map_err(Error::AbiFailed)?;
                        let target =
                            Target::for_abi(&abi).ok_or_else(|| Error::AbiInvalid(abi.clone()))?;
                        Ok(Device::new(serial_no, name, model, target))
                    })
                    .collect()
            }),
    )
    .map_err(Error::DevicesFailed)?
}
