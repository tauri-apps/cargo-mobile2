mod update;
pub(crate) mod xcode_plugin;

use self::update::{Outdated, OutdatedError};
use super::system_profile::{self, DeveloperTools};
use crate::{
    opts,
    util::{
        self,
        cli::{Report, TextWrapper},
        prompt,
    },
};
use once_cell_regex::regex;
use std::collections::hash_set::HashSet;
use thiserror::Error;

static PACKAGES: &[PackageSpec] = &[
    PackageSpec::brew("xcodegen"),
    PackageSpec::brew("ios-deploy"),
    PackageSpec::brew_or_gem("cocoapods").with_bin_name("pod"),
];

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    OutdatedFailed(#[from] OutdatedError),
    #[error("Failed to check for presence of `{package}`: {source}")]
    PresenceCheckFailed {
        package: &'static str,
        source: bossy::Error,
    },
    #[error("Failed to install `{package}`: {source}")]
    InstallFailed {
        package: &'static str,
        source: bossy::Error,
    },
    #[error("Failed to prompt to install updates: {0}")]
    PromptFailed(#[from] std::io::Error),
    #[error(transparent)]
    VersionLookupFailed(#[from] system_profile::Error),
    #[error("Failed to update package `{package}`")]
    PackageNotUpdated { package: &'static str },
    #[error("Failed to list installed gems: {0}")]
    GemListFailed(#[from] bossy::Error),
    #[error("Regex match failed for output of `gem list`")]
    RegexMatchFailed,
    #[error(transparent)]
    CaptureGroupError(#[from] util::CaptureGroupError),
}

#[derive(Default)]
pub struct GemCache {
    set: HashSet<String>,
}

impl GemCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn initialize(&mut self) -> Result<(), Error> {
        if self.set.is_empty() {
            self.set = bossy::Command::impure_parse("gem list")
                .run_and_wait_for_string()
                .map_err(Error::GemListFailed)?
                .lines()
                .flat_map(|string| {
                    regex!(r"(?P<name>.+) \(.+\)").captures(string).map(|caps| {
                        util::get_string_for_group(&caps, "name", string)
                            .map_err(Error::CaptureGroupError)
                    })
                })
                .collect::<Result<_, Error>>()?;
        }
        Ok(())
    }

    pub fn contains(&mut self, package: &str) -> Result<bool, Error> {
        self.initialize()?;
        Ok(self.contains_unchecked(package))
    }

    pub fn contains_unchecked(&self, package: &str) -> bool {
        self.set.contains(package)
    }

    pub fn reinstall(&mut self, package: &'static str) -> Result<(), Error> {
        let command = if self.contains(package)? {
            "gem update"
        } else {
            println!("`sudo` is required to install {} using gem", package);
            "sudo gem install"
        };
        bossy::Command::impure_parse(command)
            .with_arg(package)
            .run_and_wait()
            .map_err(|source| Error::InstallFailed { package, source })?;
        Ok(())
    }
}

fn installed_with_brew(package: &str) -> bool {
    bossy::Command::impure_parse("brew list")
        .with_arg(package)
        .run_and_wait_for_output()
        .is_ok()
}

fn brew_reinstall(package: &'static str) -> Result<(), Error> {
    // reinstall works even if it's not installed yet, and will upgrade
    // if it's already installed!
    bossy::Command::impure_parse("brew reinstall")
        .with_arg(package)
        .run_and_wait()
        .map_err(|source| Error::InstallFailed { package, source })?;
    Ok(())
}

fn update_package(package: &'static str, gem_cache: &mut GemCache) -> Result<(), Error> {
    if installed_with_brew(package) {
        brew_reinstall(package)?;
    } else {
        gem_cache.reinstall(package)?;
    }
    Ok(())
}

#[derive(Debug)]
pub enum PackageSource {
    Brew,
    BrewOrGem,
}

#[derive(Debug)]
pub struct PackageSpec {
    pub pkg_name: &'static str,
    pub bin_name: &'static str,
    pub package_source: PackageSource,
}

impl PackageSpec {
    pub const fn brew(pkg_name: &'static str) -> Self {
        Self {
            pkg_name,
            bin_name: pkg_name,
            package_source: PackageSource::Brew,
        }
    }

    pub const fn brew_or_gem(pkg_name: &'static str) -> Self {
        Self {
            pkg_name,
            bin_name: pkg_name,
            package_source: PackageSource::BrewOrGem,
        }
    }

    pub const fn with_bin_name(mut self, bin_name: &'static str) -> Self {
        self.bin_name = bin_name;
        self
    }

    pub fn found(&self) -> Result<bool, Error> {
        let found =
            util::command_present(self.bin_name).map_err(|source| Error::PresenceCheckFailed {
                package: self.pkg_name,
                source,
            })?;
        log::info!("package `{}` present: {}", self.pkg_name, found);
        Ok(found)
    }

    pub fn install(
        &self,
        reinstall_deps: opts::ReinstallDeps,
        gem_cache: &mut GemCache,
    ) -> Result<bool, Error> {
        if !self.found()? || reinstall_deps.yes() {
            println!("Installing `{}`...", self.pkg_name);
            match self.package_source {
                PackageSource::Brew => brew_reinstall(self.pkg_name)?,
                PackageSource::BrewOrGem => update_package(self.pkg_name, gem_cache)?,
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

pub fn install_all(
    wrapper: &TextWrapper,
    non_interactive: opts::NonInteractive,
    skip_dev_tools: opts::SkipDevTools,
    reinstall_deps: opts::ReinstallDeps,
) -> Result<(), Error> {
    let mut gem_cache = GemCache::new();
    for package in PACKAGES {
        package.install(reinstall_deps, &mut gem_cache)?;
    }
    gem_cache.initialize()?;
    let outdated = Outdated::load(&mut gem_cache)?;
    outdated.print_notice();
    if !outdated.is_empty() && non_interactive.no() {
        let answer = loop {
            if let Some(answer) = prompt::yes_no(
                "Would you like these outdated dependencies to be updated for you?",
                Some(prompt::YesOrNo::Yes),
            )? {
                break answer;
            }
        };
        if answer.yes() {
            for package in outdated.iter() {
                update_package(package, &mut gem_cache)?;
            }
        }
    }
    // we definitely don't want to install this on CI...
    if skip_dev_tools.no() {
        let tool_info = DeveloperTools::new()?;
        let result = xcode_plugin::install(wrapper, reinstall_deps, tool_info.version);
        if let Err(err) = result {
            // philosophy: never be so sturbborn as to prevent use / progress
            Report::action_request(
                "Failed to install Rust Xcode plugin; this component is optional, so init will continue anyway, but Xcode debugging won't work until this is resolved!",
                err,
            )
            .print(wrapper);
        }
    }
    Ok(())
}
