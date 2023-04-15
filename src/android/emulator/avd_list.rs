use super::Emulator;
use crate::{android::env::Env, bossy, env::ExplicitEnv as _};
use std::{collections::BTreeSet, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to run `adb devices`: {0}")]
    ListAvdsFailed(bossy::Error),
}

pub fn avd_list(env: &Env) -> Result<BTreeSet<Emulator>, Error> {
    bossy::Command::pure(PathBuf::from(env.android_home()).join("emulator/emulator"))
        .with_env_vars(env.explicit_env())
        .with_args(["-list-avds"])
        .run_and_wait_for_str(|raw_list| {
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
