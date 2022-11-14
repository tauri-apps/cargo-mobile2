use super::Section;
use crate::{android, doctor::Unrecoverable, os::Env, util};

pub fn check(env: &Env) -> Result<Section, Unrecoverable> {
    let section = Section::new("Android developer tools");
    Ok(match android::env::Env::from_env(env.clone()) {
        Ok(android_env) => section
            // It'd be a bit too inconvenient to use `map` here, since we need
            // to use `?` within the closures...
            .with_item(match android_env.sdk_version() {
                Ok(sdk_version) => Ok(format!(
                    "SDK v{} installed at {:?}",
                    sdk_version,
                    util::contract_home(android_env.android_home())?,
                )),
                Err(err) => Err(format!("Failed to get SDK version: {}", err)),
            })
            .with_item(match android_env.ndk.version() {
                Ok(ndk_version) => Ok(format!(
                    "NDK v{} installed at {:?}",
                    ndk_version,
                    util::contract_home(android_env.ndk.home())?,
                )),
                Err(err) => Err(format!("Failed to get NDK version: {}", err)),
            }),
        Err(err) => section.with_failure(err),
    })
}
