use once_cell_regex::regex;
use openssl::{
    error::ErrorStack as OpenSslError,
    nid::Nid,
    x509::{X509NameRef, X509},
};
use std::collections::BTreeSet;
use thiserror::Error;

pub fn get_pem_list(name_substr: &str) -> std::io::Result<std::process::Output> {
    duct::cmd(
        "security",
        ["find-certificate", "-p", "-a", "-c", name_substr],
    )
    .stderr_capture()
    .stdout_capture()
    .run()
}

pub fn get_pem_list_old_name_scheme() -> std::io::Result<std::process::Output> {
    get_pem_list("Developer:")
}

pub fn get_pem_list_new_name_scheme() -> std::io::Result<std::process::Output> {
    get_pem_list("Development:")
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to call `security` command: {0}")]
    SecurityCommandFailed(#[from] std::io::Error),
    #[error("Failed to parse X509 cert: {0}")]
    X509ParseFailed(#[source] OpenSslError),
}

#[derive(Debug, Error)]
pub enum X509FieldError {
    #[error("Missing X509 field {name:?} ({id:?})")]
    FieldMissing { name: &'static str, id: Nid },
    #[error("Field contained invalid UTF-8: {0}")]
    FieldNotValidUtf8(#[source] OpenSslError),
}

pub fn get_x509_field(
    subject_name: &X509NameRef,
    field_name: &'static str,
    field_nid: Nid,
) -> Result<String, X509FieldError> {
    subject_name
        .entries_by_nid(field_nid)
        .next()
        .ok_or(X509FieldError::FieldMissing {
            name: field_name,
            id: field_nid,
        })?
        .data()
        .as_utf8()
        .map_err(X509FieldError::FieldNotValidUtf8)
        .map(|s| s.to_string())
}

#[derive(Debug, Error)]
pub enum FromX509Error {
    #[error("skipping cert: {0}")]
    CommonNameMissing(#[source] X509FieldError),
    #[error("skipping cert {common_name:?}: {source}")]
    OrganizationalUnitMissing {
        common_name: String,
        source: X509FieldError,
    },
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Team {
    pub name: String,
    pub id: String,
}

impl Team {
    pub fn from_x509(cert: X509) -> Result<Self, FromX509Error> {
        let subj = cert.subject_name();
        let common_name = get_x509_field(subj, "Common Name", Nid::COMMONNAME)
            .map_err(FromX509Error::CommonNameMissing)?;
        let organization = get_x509_field(subj, "Organization", Nid::ORGANIZATIONNAME);
        let name = if let Ok(organization) = organization {
            log::debug!(
                "found cert {:?} with organization {:?}",
                common_name,
                organization
            );
            organization
        } else {
            log::debug!(
                "found cert {:?} but failed to get organization; falling back to displaying common name",
                common_name
            );
            regex!(r"Apple Develop\w+: (.*) \(.+\)")
                .captures(&common_name)
                .map(|caps| caps[1].to_owned())
                .unwrap_or_else(|| {
                    log::debug!("regex failed to capture nice part of name in cert {:?}; falling back to displaying full name", common_name);
                    common_name.clone()
                })
        };
        let id = get_x509_field(subj, "Organizational Unit", Nid::ORGANIZATIONALUNITNAME).map_err(
            |source| FromX509Error::OrganizationalUnitMissing {
                common_name,
                source,
            },
        )?;
        Ok(Self { name, id })
    }
}

pub fn find_development_teams() -> Result<Vec<Team>, Error> {
    let certs = {
        let new = get_pem_list_new_name_scheme().map_err(Error::SecurityCommandFailed)?;
        let mut certs = X509::stack_from_pem(&new.stdout).map_err(Error::X509ParseFailed)?;
        let old = get_pem_list_old_name_scheme().map_err(Error::SecurityCommandFailed)?;
        certs.append(&mut X509::stack_from_pem(&old.stdout).map_err(Error::X509ParseFailed)?);
        certs
    };
    Ok(certs
        .into_iter()
        .flat_map(|cert| {
            Team::from_x509(cert).map_err(|err| {
                log::error!("{}", err);
                err
            })
        })
        // Silly way to sort this and ensure no dupes
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect())
}
