use crate::teams;
use ginit_core::{
    config::{empty, RawConfigTrait},
    exports::colored::*,
    util,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::{self, Display},
};

#[derive(Debug)]
pub enum DetectedError {
    DeveloperTeamLookupFailed(teams::Error),
    DeveloperTeamsEmpty,
}

impl Display for DetectedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeveloperTeamLookupFailed(err) => {
                write!(f, "Failed to find Apple developer teams: {}", err)
            }
            Self::DeveloperTeamsEmpty => write!(f, "No Apple developer teams were detected."),
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Raw {
    #[serde(alias = "development-team")]
    pub development_team: String,
    #[serde(alias = "project-path")]
    pub project_path: Option<String>,
    pub targets: Option<HashMap<String, HashMap<String, String>>>,
}

impl RawConfigTrait for Raw {
    type Detected = empty::Detected;

    type FromDetectedError = DetectedError;
    fn from_detected(_detected: Self::Detected) -> Result<Self, Self::FromDetectedError> {
        let development_teams =
            teams::find_development_teams().map_err(DetectedError::DeveloperTeamLookupFailed)?;
        Ok(Self {
            development_team: development_teams
                .get(0)
                .map(|development_team| development_team.id.clone())
                .ok_or_else(|| DetectedError::DeveloperTeamsEmpty)?,
            project_path: None,
            targets: None,
        })
    }

    type FromPromptError = PromptError;
    fn from_prompt(
        _detected: Self::Detected,
        wrapper: &util::TextWrapper,
    ) -> Result<Self, Self::FromPromptError> {
        let development_team = {
            let development_teams =
                teams::find_development_teams().map_err(PromptError::DeveloperTeamLookupFailed)?;
            let mut default_team = None;
            println!("Detected development teams:");
            for (index, team) in development_teams.iter().enumerate() {
                println!(
                    "  [{}] {} ({})",
                    index.to_string().green(),
                    team.name,
                    team.id.cyan(),
                );
                if development_teams.len() == 1 {
                    default_team = Some("0");
                }
            }
            if development_teams.is_empty() {
                println!("  -- none --");
            }
            let mut development_team = None;
            while let None = development_team {
                println!(
                    "  Enter an {} for a team above, or enter a {} manually.",
                    "index".green(),
                    "team ID".cyan(),
                );
                let team_input = util::prompt::default(
                    "Apple development team",
                    default_team,
                    Some(Color::Green),
                )
                .map_err(PromptError::DeveloperTeamPromptFailed)?;
                let team_id = team_input
                    .parse::<usize>()
                    .ok()
                    .and_then(|index| development_teams.get(index))
                    .map(|team| team.id.clone())
                    .unwrap_or_else(|| team_input);
                if !team_id.is_empty() {
                    development_team = Some(team_id);
                } else {
                    println!(
                        "{}",
                        wrapper
                            .fill("Uh-oh, you need to specify a development team ID.")
                            .bright_magenta()
                    );
                }
            }
            development_team.unwrap()
        };
        Ok(Self {
            development_team,
            project_path: None,
            targets: None,
        })
    }
}
