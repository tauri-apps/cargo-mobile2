use into_result::{
    command::{CommandError, CommandResult},
    IntoResult as _,
};
use openssl::{
    error::ErrorStack as OpenSslError,
    nid::Nid,
    x509::{X509NameRef, X509},
};
use std::{collections::BTreeSet, process::Command};

pub fn get_pem_list() -> CommandResult<Vec<u8>> {
    Command::new("security")
        .args(&["find-certificate", "-p", "-a", "-c", "Developer:"])
        .output()
        .into_result()
        .map(|output| output.stdout)
}

#[derive(Debug, derive_more::From)]
pub enum FindTeamsError {
    FindCertsError(CommandError),
    ParseX509Error(OpenSslError),
    MissingX509Field(Nid),
    AsUtf8Error(OpenSslError),
}

pub fn get_x509_field(name: &X509NameRef, nid: Nid) -> Result<String, FindTeamsError> {
    name.entries_by_nid(nid)
        .nth(0)
        .ok_or(FindTeamsError::MissingX509Field(nid))?
        .data()
        .as_utf8()
        .map_err(FindTeamsError::AsUtf8Error)
        .map(|s| s.to_string())
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Team {
    pub name: String,
    pub id: String,
}

impl Team {
    pub fn from_x509(cert: X509) -> Result<Self, FindTeamsError> {
        let subj = cert.subject_name();
        let name = get_x509_field(subj, Nid::ORGANIZATIONNAME)?;
        let id = get_x509_field(subj, Nid::ORGANIZATIONALUNITNAME)?;
        Ok(Self { name, id })
    }
}

pub fn find_development_teams() -> Result<Vec<Team>, FindTeamsError> {
    let certs = X509::stack_from_pem(&get_pem_list()?).map_err(FindTeamsError::ParseX509Error)?;
    let mut teams = BTreeSet::new();
    for cert in certs {
        teams.insert(Team::from_x509(cert)?);
    }
    Ok(teams.into_iter().collect())
}
