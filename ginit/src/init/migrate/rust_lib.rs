use super::type_state::*;
use crate::{config::Config, util};
use into_result::command::CommandResult;
use std::marker::PhantomData;

#[derive(Debug)]
pub struct RustLib<T> {
    _marker: PhantomData<T>,
}

impl RustLib<Legacy> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }

    pub fn move_to_root(self, config: &Config) -> CommandResult<RustLib<Moved>> {
        let old = config.app_root().join("rust/lib");
        let new = config.app_root().join("rust-lib");
        util::git(
            &config.app_root(),
            &["mv".as_ref(), old.as_os_str(), new.as_os_str()],
        )?;
        Ok(RustLib {
            _marker: PhantomData,
        })
    }
}
