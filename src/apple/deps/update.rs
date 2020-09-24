use super::PACKAGES;
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OutdatedError {
    #[error("Failed to check for outdated packages: {0}")]
    CommandFailed(#[from] bossy::Error),
    #[error("Failed to parse outdated package list: {0}")]
    ParseFailed(#[from] serde_json::Error),
}

#[derive(Debug, Deserialize)]
struct Formula {
    name: String,
    installed_versions: Vec<String>,
    current_version: String,
    // pinned: bool,
    // pinned_version: Option<String>,
}

impl Formula {
    fn print_notice(&self) {
        if self.installed_versions.len() == 1 {
            println!(
                "  - `{}` is at {}; latest version is {}",
                self.name, self.installed_versions[0], self.current_version
            );
        } else {
            println!(
                "  - `{}` is at [{}]; latest version is {}",
                self.name,
                self.installed_versions.join(", "),
                self.current_version
            );
        }
    }
}

#[derive(Debug)]
pub struct Outdated {
    packages: Vec<Formula>,
}

impl Outdated {
    pub fn load() -> Result<Self, OutdatedError> {
        #[derive(Deserialize)]
        struct Raw {
            formulae: Vec<Formula>,
        }

        bossy::Command::impure_parse("brew outdated --json=v2")
            .run_and_wait_for_output()
            .map_err(Into::into)
            .and_then(|output| serde_json::from_slice(output.stdout()).map_err(Into::into))
            .map(|Raw { formulae }| Self {
                packages: formulae
                    .into_iter()
                    .filter(|formula| PACKAGES.contains(&formula.name.as_str()))
                    .collect(),
            })
    }

    pub fn iter(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.packages.iter().map(|formula| {
            PACKAGES
                .iter()
                // Do a switcheroo to get static lifetimes, just for the dubious
                // goal of not needing to use `String` in `deps::Error`...
                .find(|package| **package == formula.name.as_str())
                .copied()
                .expect("developer error: outdated package list should be a subset of `PACKAGES`")
        })
    }

    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }

    pub fn print_notice(&self) {
        if !self.is_empty() {
            println!("Outdated dependencies:");
            for package in self.packages.iter() {
                package.print_notice();
            }
        } else {
            println!("Apple dependencies are up to date");
        }
    }
}
