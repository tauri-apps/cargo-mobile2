use crate::{
    apple::teams,
    util::{cli::TextWrapper, prompt},
};
use colored::{Color, Colorize as _};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug, Display};

#[derive(Debug)]
pub enum DetectError {
    DeveloperTeamLookupFailed(teams::Error),
}

impl Display for DetectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeveloperTeamLookupFailed(err) => {
                write!(f, "Failed to find Apple developer teams: {}", err)
            }
        }
    }
}

#[derive(Debug)]
pub enum PromptError {
    DeveloperTeamLookupFailed(teams::Error),
    DeveloperTeamPromptFailed(std::io::Error),
}

impl Display for PromptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeveloperTeamLookupFailed(err) => {
                write!(f, "Failed to find Apple developer teams: {}", err)
            }
            Self::DeveloperTeamPromptFailed(err) => {
                write!(f, "Failed to prompt for Apple developer team: {}", err)
            }
        }
    }
}

fn value_to_string(value: &PlistValue) -> String {
    match value {
        PlistValue::Bool(bool) => bool.to_string(),
        PlistValue::String(string) => string.to_owned(),
        PlistValue::Array(array) => {
            let string = array
                .iter()
                .map(value_to_string)
                .collect::<Vec<_>>()
                .join(",");
            format!("[{}]", string)
        }
        PlistValue::Dictionary(dict) => dictionary_to_string(dict),
    }
}

fn pair_to_string(key: &str, value: &PlistValue) -> String {
    format!("{}: {}", key, value_to_string(value))
}

fn dictionary_to_string(dict: &PlistDictionary) -> String {
    let joint = dict
        .dictionary
        .iter()
        .map(|pair| pair_to_string(&pair.0.key, &pair.0.value))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{}}}", joint)
}

#[derive(Clone, Debug, Deserialize)]
#[serde(transparent)]
pub struct NestedPair(PListPair);

impl Serialize for NestedPair {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&pair_to_string(&self.0.key, &self.0.value))
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct PlistDictionary {
    dictionary: Vec<NestedPair>,
}

impl Serialize for PlistDictionary {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&dictionary_to_string(self))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PlistValue {
    Bool(bool),
    String(String),
    Array(Vec<PlistValue>),
    Dictionary(PlistDictionary),
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PListPair {
    key: String,
    value: PlistValue,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Raw {
    pub development_team: Option<String>,
    pub project_dir: Option<String>,
    pub ios_no_default_features: Option<bool>,
    pub ios_features: Option<Vec<String>>,
    pub macos_no_default_features: Option<bool>,
    pub macos_features: Option<Vec<String>>,
    pub bundle_version: Option<String>,
    pub bundle_version_short: Option<String>,
    pub ios_version: Option<String>,
    pub macos_version: Option<String>,
    pub use_legacy_build_system: Option<bool>,
    pub plist_pairs: Option<Vec<PListPair>>,
    pub enable_bitcode: Option<bool>,
}

impl Raw {
    pub fn detect() -> Result<Self, DetectError> {
        let development_teams =
            teams::find_development_teams().map_err(DetectError::DeveloperTeamLookupFailed)?;
        Ok(Self {
            development_team: development_teams
                .first()
                .map(|development_team| development_team.id.clone()),
            project_dir: None,
            ios_no_default_features: None,
            ios_features: None,
            macos_no_default_features: None,
            macos_features: None,
            bundle_version: None,
            bundle_version_short: None,
            ios_version: None,
            macos_version: None,
            use_legacy_build_system: None,
            plist_pairs: None,
            enable_bitcode: None,
        })
    }

    pub fn prompt(wrapper: &TextWrapper) -> Result<Self, PromptError> {
        let development_team = {
            let development_teams =
                teams::find_development_teams().map_err(PromptError::DeveloperTeamLookupFailed)?;
            let default_team = if !development_teams.is_empty() {
                Some("0")
            } else {
                None
            };
            println!("Detected development teams:");
            for (index, team) in development_teams.iter().enumerate() {
                if index == 0 {
                    println!(
                        "{}",
                        format!(
                            "  [{}] {} ({})",
                            index.to_string().bright_green(),
                            team.name,
                            team.id.bright_cyan(),
                        )
                        .bright_white()
                        .bold()
                    );
                } else {
                    println!(
                        "  [{}] {} ({})",
                        index.to_string().green(),
                        team.name,
                        team.id.cyan(),
                    );
                }
            }
            if development_teams.is_empty() {
                println!("  -- none --");
            }
            loop {
                println!(
                    "  Enter an {} for a team above, or enter a {} manually.",
                    "index".green(),
                    "team ID".cyan(),
                );
                let team_input = prompt::default(
                    "Apple development team",
                    default_team,
                    Some(Color::BrightGreen),
                )
                .map_err(PromptError::DeveloperTeamPromptFailed)?;
                let team_id = team_input
                    .parse::<usize>()
                    .ok()
                    .and_then(|index| development_teams.get(index))
                    .map(|team| team.id.clone())
                    .unwrap_or_else(|| team_input);
                if !team_id.is_empty() {
                    break team_id;
                } else {
                    println!(
                        "{}",
                        wrapper
                            .fill("Uh-oh, you need to specify a development team ID.")
                            .bright_magenta()
                    );
                }
            }
        };
        Ok(Self {
            development_team: Some(development_team),
            project_dir: None,
            ios_no_default_features: None,
            ios_features: None,
            macos_no_default_features: None,
            macos_features: None,
            bundle_version: None,
            bundle_version_short: None,
            ios_version: None,
            macos_version: None,
            use_legacy_build_system: None,
            plist_pairs: None,
            enable_bitcode: None,
        })
    }
}
