use super::{Item, Section};
use crate::{
    apple::{deps::xcode_plugin, system_profile::DeveloperTools, teams},
    util::prompt,
    DuctExpressionExt,
};
use std::path::Path;

fn validate_developer_dir() -> Result<String, String> {
    static FORBIDDEN: &str = "/Library/Developer/CommandLineTools";
    static SUGGESTED: &str = "/Applications/Xcode.app/Contents/Developer";
    let xcode_developer_dir = xcode_plugin::xcode_developer_dir()
        .map_err(|err| format!("Failed to get active Xcode developer dir: {}", err))?;
    let xcode_developer_dir = {
        if xcode_developer_dir == Path::new(FORBIDDEN) {
            println!(
                "Your active toolchain appears to be the Apple command-line tools: {:?}",
                xcode_developer_dir
            );
            println!("Changing your active toolchain to Xcode may be necessary for everything to work correctly.");
            let answer = loop {
                if let Some(answer) = prompt::yes_no(
                    format!("Would you like us to change it to {:?} for you?", SUGGESTED),
                    Some(true),
                )
                .map_err(|err| {
                    format!(
                        "Failed to prompt for changing the Xcode developer dir: {}",
                        err
                    )
                })? {
                    break answer;
                }
            };
            if answer {
                duct::cmd("xcode-select", ["-s", SUGGESTED])
                    .dup_stdio()
                    .run()
                    .map_err(|err| format!("Failed to update Xcode developer dir: {}", err))?;
                Path::new(SUGGESTED)
            } else {
                &xcode_developer_dir
            }
        } else {
            &xcode_developer_dir
        }
    };
    Ok(format!("Active developer dir: {:?}", xcode_developer_dir))
}

fn validate_xcode_plugin(xcode_version: (u32, u32), section: Section) -> Section {
    match xcode_plugin::Context::new(xcode_version) {
        Ok(ctx) => match ctx.check_installation() {
            Ok(status) => section
                .with_item(if status.plugin_present {
                    Item::victory("xcode-rust-plugin plugin present")
                } else {
                    Item::warning("xcode-rust-plugin plugin absent")
                })
                .with_item(if status.lang_spec_present {
                    Item::victory("xcode-rust-plugin lang spec present")
                } else {
                    Item::warning("xcode-rust-plugin lang spec absent")
                })
                .with_item(if status.lang_metadata_present {
                    Item::victory("xcode-rust-plugin lang metadata present")
                } else {
                    Item::warning("xcode-rust-plugin lang metadata absent")
                })
                .with_item(if status.repo_fresh {
                    Item::victory("xcode-rust-plugin is up-to-date")
                } else {
                    Item::warning("xcode-rust-plugin is outdated")
                }),
            Err(err) => section.with_failure(format!(
                "Failed to check xcode-rust-plugin installation status: {}",
                err
            )),
        }
        .with_item(match ctx.check_uuid() {
            Ok(status) => {
                if status.supported {
                    Item::victory(format!(
                        "xcode-rust-plugin supports Xcode UUID {:?}",
                        status.uuid
                    ))
                } else {
                    Item::warning(format!(
                        "xcode-rust-plugin doesn't support Xcode UUID {:?}",
                        status.uuid
                    ))
                }
            }
            Err(err) => Item::failure(format!(
                "Failed to check xcode-rust-plugin UUID status: {}",
                err
            )),
        }),
        Err(err) => {
            section.with_failure(format!("Failed to get xcode-rust-plugin context: {}", err))
        }
    }
}

pub fn check() -> Section {
    let xcode_version = DeveloperTools::new().map(|dev_tools| dev_tools.version);
    let section = Section::new("Apple developer tools")
        .with_item(
            xcode_version
                .as_ref()
                .map(|(major, minor)| format!("Xcode v{}.{}", major, minor))
                .map_err(|err| format!("Failed to check Xcode version: {}", err)),
        )
        .with_item(validate_developer_dir())
        .with_item(
            duct::cmd("ios-deploy", ["--version"])
                .stderr_capture()
                .read()
                .map(|version| format!("ios-deploy v{}", version.trim()))
                .map_err(|err| format!("Failed to check ios-deploy version: {}", err)),
        )
        .with_item(
            duct::cmd("xcodegen", ["--version"])
                .stderr_capture()
                .read()
                .map(|version| version.trim().replace("Version: ", "XcodeGen v"))
                .map_err(|err| format!("Failed to check ios-deploy version: {}", err)),
        );
    let section = if let Ok(version) = xcode_version {
        validate_xcode_plugin(version, section)
    } else {
        section
    };
    match teams::find_development_teams() {
        Ok(teams) => {
            section.with_victories(teams.into_iter().map(|team| {
                // TODO: improve development/developer consistency throughout
                // cargo-mobile2
                format!("Development team: {} ({})", team.name, team.id)
            }))
        }
        Err(err) => section.with_failure(format!("Failed to find development teams: {}", err)),
    }
}
