use super::type_state::*;
use crate::config::Config;
use std::{fs, io, marker::PhantomData};

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

    pub fn move_to_root(self, config: &Config) -> io::Result<RustLib<Moved>> {
        fs::rename(
            config.app_root().join("rust/lib"),
            config.app_root().join("rust-lib"),
        )?;
        Ok(RustLib {
            _marker: PhantomData,
        })
    }
}
