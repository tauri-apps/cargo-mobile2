use once_cell_regex::regex;
use openssl::{
    error::ErrorStack as OpenSslError,
    nid::Nid,
    x509::{X509NameRef, X509},
};
use std::collections::BTreeSet;
use thiserror::Error;

pub fn get_pem_list(name_substr: &str) -> bossy::Result<bossy::Output> {
    bossy::Command::impure("security")
        .with_args(&["find-certificate", "-p", "-a", "-c", name_substr])
        .run_and_wait_for_output()
}

pub fn get_pem_list_old_name_scheme() -> bossy::Result<bossy::Output> {
    get_pem_list("Developer:")
}

pub fn get_pem_list_new_name_scheme() -> bossy::Result<bossy::Output> {
    get_pem_list("Development:")
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to call `security` command: {0}")]
    SecurityCommandFailed(#[from] bossy::Error),
    #[error("Failed to parse X509 cert: {0}")]
    X509ParseFailed(#[source] OpenSslError),
    #[error("Missing X509 field {name:?} ({id:?})")]
    X509FieldMissing { name: &'static str, id: Nid },
    #[error("Field contained invalid UTF-8: {0}")]
    FieldNotValidUtf8(#[source] OpenSslError),
}

pub fn get_x509_field(
    subject_name: &X509NameRef,
    field_name: &'static str,
    field_nid: Nid,
) -> Result<String, Error> {
    subject_name
        .entries_by_nid(field_nid)
        .nth(0)
        .ok_or(Error::X509FieldMissing {
            name: field_name,
            id: field_nid,
        })?
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
        let common_name = get_x509_field(subj, "Common Name", Nid::COMMONNAME)?;
        let organization = get_x509_field(subj, "Organization", Nid::ORGANIZATIONNAME);
        let name = if let Ok(organization) = organization {
            log::info!(
                "found cert {:?} with organization {:?}",
                common_name,
                organization
            );
            organization
        } else {
            log::error!(
                "found cert {:?} but failed to get organization; falling back to displaying common name",
                common_name
            );
            regex!(r"Apple Develop\w+: (.*) \(.+\)")
                .captures(&common_name)
                .map(|caps| caps[1].to_owned())
                .unwrap_or_else(|| {
                    log::error!("regex failed to capture nice part of name in cert {:?}; falling back to displaying full name", common_name);
                    common_name
                })
        };
        let id = get_x509_field(subj, "Organizationl Unit", Nid::ORGANIZATIONALUNITNAME)?;
        Ok(Self { name, id })
    }
}

pub fn find_development_teams() -> Result<Vec<Team>, Error> {
    let certs = {
        let new = get_pem_list_new_name_scheme().map_err(Error::SecurityCommandFailed)?;
        let mut certs = X509::stack_from_pem(new.stdout()).map_err(Error::X509ParseFailed)?;
        let old = get_pem_list_old_name_scheme().map_err(Error::SecurityCommandFailed)?;
        certs.append(&mut X509::stack_from_pem(old.stdout()).map_err(Error::X509ParseFailed)?);
        certs
    };
    let mut teams = BTreeSet::new();
    for cert in certs {
        teams.insert(Team::from_x509(cert)?);
    }
    Ok(teams.into_iter().collect())
}
