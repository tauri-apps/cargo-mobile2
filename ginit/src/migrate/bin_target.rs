use super::type_state::*;
use crate::config::Config;
use std::{fs, io, marker::PhantomData};

#[derive(Debug)]
pub struct BinTarget<T> {
    _marker: PhantomData<T>,
}

impl BinTarget<Legacy> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }

    pub fn move_to_gen(self, config: &Config) -> io::Result<BinTarget<Moved>> {
        let desktop_crate = config
            .app_root()
            .join(format!("rust/{}-desktop/src", config.app_name()));
        let bin_dir = config.app_root().join("gen/bin");
        fs::rename(desktop_crate, &bin_dir)?;
        fs::rename(bin_dir.join("main.rs"), bin_dir.join("desktop.rs"))?;
        Ok(BinTarget {
            _marker: PhantomData,
        })
    }
}
