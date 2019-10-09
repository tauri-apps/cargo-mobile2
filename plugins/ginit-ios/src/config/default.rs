use super::required::RequiredConfig;
use crate::teams;
use ginit_core::{config::DefaultConfigTrait, util};
use std::fmt::{self, Display};

#[derive(Debug)]
pub enum UpgradeError {
    DeveloperTeamLookupFailed(teams::Error),
    DeveloperTeamsEmpty,
}

impl Display for UpgradeError {
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
pub struct DefaultConfig;

impl DefaultConfigTrait for DefaultConfig {
    type DetectError = util::Never;
    fn detect() -> Result<Self, Self::DetectError> {
        Ok(Self)
    }

    type RequiredConfig = RequiredConfig;
    type UpgradeError = UpgradeError;
    fn upgrade(self) -> Result<Self::RequiredConfig, Self::UpgradeError> {
        let development_teams =
            teams::find_development_teams().map_err(UpgradeError::DeveloperTeamLookupFailed)?;
        Ok(RequiredConfig {
            development_team: development_teams
                .get(0)
                .map(|development_team| development_team.id.clone())
                .ok_or_else(|| UpgradeError::DeveloperTeamsEmpty)?,
        })
    }
}
