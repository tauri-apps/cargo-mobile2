use super::Emulator;
use crate::{android::env::Env, env::ExplicitEnv, DuctExpressionExt};
use std::{collections::BTreeSet, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to run `adb devices`: {0}")]
    ListAvdsFailed(std::io::Error),
}

pub fn avd_list(env: &Env) -> Result<BTreeSet<Emulator>, Error> {
    duct::cmd(
        PathBuf::from(env.android_home()).join("emulator/emulator"),
        ["-list-avds"],
    )
    .vars(env.explicit_env())
    .stderr_capture()
    .read()
    .map(|raw_list| {
        raw_list
            .split('\n')
            .filter_map(|name| {
                if name.is_empty() {
                    None
                } else {
                    Some(Emulator::new(name.trim().into()))
                }
            })
            .collect::<BTreeSet<Emulator>>()
    })
    .map_err(Error::ListAvdsFailed)
}
