use into_result::{
    command::{CommandError, CommandResult},
    IntoResult as _,
};
use openssl::{
    error::ErrorStack as OpenSslError,
    nid::Nid,
    x509::{X509NameRef, X509},
};
use std::{collections::BTreeSet, fmt, process::Command};

fn get_pem_list(name_substr: &str) -> CommandResult<Vec<u8>> {
    Command::new("security")
        .args(&["find-certificate", "-p", "-a", "-c", name_substr])
        .output()
        .into_result()
        .map(|output| output.stdout)
}

pub fn get_pem_list_old_name_scheme() -> CommandResult<Vec<u8>> {
    get_pem_list("Developer:")
}

pub fn get_pem_list_new_name_scheme() -> CommandResult<Vec<u8>> {
    get_pem_list("Development:")
}

#[derive(Debug)]
pub enum Error {
    SecurityCommandFailed(CommandError),
    X509ParseFailed(OpenSslError),
    X509FieldMissing(Nid),
    FieldNotValidUtf8(OpenSslError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::SecurityCommandFailed(err) => {
                write!(f, "Failed to call `security` command: {}", err)
            }
            Error::X509ParseFailed(err) => write!(f, "Failed to parse X509 cert: {}", err),
            Error::X509FieldMissing(nid) => write!(f, "Missing X509 field: {:?}", nid),
            Error::FieldNotValidUtf8(err) => write!(f, "Field contained invalid UTF-8: {}", err),
        }
    }
}

pub fn get_x509_field(name: &X509NameRef, nid: Nid) -> Result<String, Error> {
    name.entries_by_nid(nid)
        .nth(0)
        .ok_or(Error::X509FieldMissing(nid))?
        .data()
        .as_utf8()
        .map_err(Error::FieldNotValidUtf8)
        .map(|s| s.to_string())
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Team {
    pub name: String,
    pub id: String,
}

impl Team {
    pub fn from_x509(cert: X509) -> Result<Self, Error> {
        let subj = cert.subject_name();
        let name = get_x509_field(subj, Nid::ORGANIZATIONNAME)?;
        let id = get_x509_field(subj, Nid::ORGANIZATIONALUNITNAME)?;
        Ok(Self { name, id })
    }
}

pub fn find_development_teams() -> Result<Vec<Team>, Error> {
    let certs = {
        let new = get_pem_list_new_name_scheme().map_err(Error::SecurityCommandFailed)?;
        let mut certs = X509::stack_from_pem(&new).map_err(Error::X509ParseFailed)?;
        let old = get_pem_list_old_name_scheme().map_err(Error::SecurityCommandFailed)?;
        certs.append(&mut X509::stack_from_pem(&old).map_err(Error::X509ParseFailed)?);
        certs
    };
    let mut teams = BTreeSet::new();
    for cert in certs {
        teams.insert(Team::from_x509(cert)?);
    }
    Ok(teams.into_iter().collect())
}
