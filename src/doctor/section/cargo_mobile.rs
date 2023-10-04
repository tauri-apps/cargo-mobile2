use super::Section;
use crate::{
    doctor::Unrecoverable,
    os,
    util::{self, cli::VERSION_SHORT},
};

fn check_os() -> Result<String, String> {
    os::Info::check()
        .map(|info| format!("{} v{}", info.name, info.version))
        .map_err(|err| format!("Failed to get OS info: {}", err))
}

fn check_rust() -> Result<String, String> {
    util::RustVersion::check()
        .map_err(|err| err.to_string())
        .and_then(|version| {
            version
                .valid()
                .then(|| format!("rustc v{}", version))
                .ok_or_else(|| {
                    format!(
                        "iOS linking is broken on rustc v{}; please update to 1.49.0 or later",
                        version
                    )
                })
        })
}

pub fn check() -> Result<Section, Unrecoverable> {
    let section = Section::new(format!("cargo-mobile {}", VERSION_SHORT));
    Ok(match util::install_dir() {
        Ok(install_dir) => section
            .with_item(util::installed_commit_msg().map(|msg| {
                msg.map(util::format_commit_msg)
                    .unwrap_or_else(|| "Installed commit message isn't present".to_string())
            }))
            .with_item(if install_dir.exists() {
                Ok(format!(
                    "Installed at {:?}",
                    util::contract_home(&install_dir)?,
                ))
            } else {
                Err(format!(
                    "The cargo-mobile2 installation directory is missing! Checked at {:?}",
                    install_dir,
                ))
            }),
        Err(err) => section.with_failure(err),
    }
    .with_item(check_os())
    .with_item(check_rust()))
}
