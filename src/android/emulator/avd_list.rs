use super::Emulator;
use crate::{android::env::Env, bossy, env::ExplicitEnv as _};
use std::collections::BTreeSet;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to run `adb devices`: {0}")]
    ListAvdsFailed(bossy::Error),
}

pub fn avd_list(env: &Env) -> Result<BTreeSet<Emulator>, Error> {
    bossy::Command::pure("emulator/emulator")
        .with_current_dir(env.sdk_root())
        .with_env_vars(env.explicit_env())
        .with_args(&["-list-avds"])
        .run_and_wait_for_str(|raw_list| {
            raw_list
                .split('\n')
                .map(|name| Emulator::new(name.into()))
                .collect::<BTreeSet<Emulator>>()
        })
        .map_err(Error::ListAvdsFailed)
}
