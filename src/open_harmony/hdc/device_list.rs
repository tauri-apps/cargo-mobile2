use super::get_prop;
use crate::{
    env::ExplicitEnv as _,
    open_harmony::{device::Device, env::Env, target::Target},
    util::cli::{Report, Reportable},
};
use std::{collections::BTreeSet, process::Command};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to run `hdc devices`: {0}")]
    DevicesFailed(#[from] super::RunCheckedError),
    #[error(transparent)]
    PropFailed(get_prop::Error),
    #[error("{0:?} isn't a valid target ABI.")]
    AbiInvalid(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Reportable for Error {
    fn report(&self) -> Report {
        let msg = "Failed to detect connected OpenHarmony devices";
        match self {
            Self::DevicesFailed(err) => err.report("Failed to run `hdc devices`"),
            Self::PropFailed(err) => err.report(),
            Self::AbiInvalid(_) => Report::error(msg, self),
            Self::Io(err) => Report::error(msg, err),
        }
    }
}

pub fn device_list(env: &Env) -> Result<BTreeSet<Device<'static>>, Error> {
    let mut cmd = Command::new(env.toolchains_path().join("hdc"));
    cmd.arg("list").arg("targets").envs(env.explicit_env());

    super::check_authorized(&cmd.output()?)
        .map(|raw_list| {
            if raw_list.trim() == "[Empty]" {
                return Ok(BTreeSet::new());
            }
            raw_list
                .trim()
                .split('\n')
                .map(|id| {
                    let model =
                        get_prop(env, &id, "const.product.model").map_err(Error::PropFailed)?;
                    let name = get_prop(
                        env,
                        &id,
                        if model.starts_with("emulator") {
                            "ohos.qemu.hvd.name"
                        } else {
                            "const.product.name"
                        },
                    )
                    .unwrap_or_else(|_| id.to_owned());
                    let abi = get_prop(env, &id, "const.product.cpu.abilist")
                        .map_err(Error::PropFailed)?;
                    let target =
                        Target::for_abi(&abi).ok_or_else(|| Error::AbiInvalid(abi.clone()))?;

                    Ok(Device::new(id.to_owned(), name, model, target))
                })
                .collect()
        })
        .map_err(Error::DevicesFailed)?
}
