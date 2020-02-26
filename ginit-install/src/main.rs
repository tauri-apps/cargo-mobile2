mod plugin;

use ginit_core::{
    dot,
    exports::{bicycle, toml},
    util::cli::NonZeroExit,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn create_dirs(dot_files: &bundle::Files) -> Result<(), NonZeroExit> {
    fs::create_dir_all(dot_files.plugins()).map_err(NonZeroExit::display)
}

fn install_bin(dot_files: &bundle::Files) -> Result<(), NonZeroExit> {
    let dest = dot_files.bin();
    todo!()
}

fn write_global_config(dot_files: &bundle::Files) -> Result<(), NonZeroExit> {
    let ser = toml::to_string_pretty(&bundle::GlobalConfig {
        default_plugins: vec![
            "brainium".to_owned(),
            "android".to_owned(),
            "ios".to_owned(),
        ],
    })
    .map_err(NonZeroExit::display)?;
    fs::write(dot_files.global_config(), ser.as_bytes()).map_err(NonZeroExit::display)
}

fn main() {
    NonZeroExit::main(|_wrapper| {
        let dot_files = bundle::Files::new().map_err(NonZeroExit::display)?;
        create_dirs(&dot_files)?;
        install_bin(&dot_files)?;
        write_global_config(&dot_files)?;
        Ok(())
    })
}
