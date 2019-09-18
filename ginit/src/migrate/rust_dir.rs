use super::{bin_target::BinTarget, rust_lib::RustLib, type_state::*, user_code::UserCode};
use crate::config::Config;
use std::{fs, io, marker::PhantomData};

#[derive(Debug)]
pub struct RustDir<T> {
    _marker: PhantomData<T>,
}

impl RustDir<Legacy> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }

    pub fn delete(
        self,
        config: &Config,
        _: &BinTarget<Moved>,
        _: &RustLib<Moved>,
        _: &UserCode<Moved>,
    ) -> io::Result<RustDir<Deleted>> {
        fs::remove_dir_all(config.app_root().join("rust"))?;
        Ok(RustDir {
            _marker: PhantomData,
        })
    }
}
