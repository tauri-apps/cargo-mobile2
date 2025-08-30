use once_cell_regex::regex;

use std::collections::BTreeSet;
use thiserror::Error;
use x509_certificate::{certificate::X509Certificate, X509CertificateError};

pub fn get_pem_list(name_substr: &str) -> duct::Expression {
    duct::cmd(
        "security",
        ["find-certificate", "-p", "-a", "-c", name_substr],
    )
    .stderr_capture()
    .stdout_capture()
}

pub fn get_pem_list_old_name_scheme() -> duct::Expression {
    get_pem_list("Developer:")
}

pub fn get_pem_list_new_name_scheme() -> duct::Expression {
    get_pem_list("Development:")
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to run `security` command: {command}: {error}")]
    SecurityCommandFailed {
        command: String,
        error: std::io::Error,
    },
    #[error("Failed to parse X509 cert: {0}")]
    X509ParseFailed(#[source] X509CertificateError),
}

#[derive(Debug, Error)]
pub enum X509FieldError {
    //#[error("Missing X509 field {name:?} ({id:?})")]
    //FieldMissing { name: &'static str, id: Nid },
    #[error("Field contained invalid UTF-8: {0}")]
    FieldNotValidUtf8(#[source] X509CertificateError),
}
/*
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
*/

#[derive(Debug, Error)]
pub enum FromX509Error {
    #[error("skipping cert, missing common name")]
    CommonNameMissing,
    #[error("skipping cert {common_name}: missing Organization Unit")]
    OrganizationalUnitMissing { common_name: String },
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Team {
    pub name: String,
    pub id: String,
}

impl Team {
    pub fn from_x509(cert: X509Certificate) -> Result<Self, FromX509Error> {
        let common_name = cert
            .subject_common_name()
            .ok_or(FromX509Error::CommonNameMissing)?;

        let organization = cert
            .subject_name()
            .iter_organization()
            .next()
            .and_then(|v| v.to_string().ok());

        let name = if let Some(organization) = organization {
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

        let id = cert
            .subject_name()
            .iter_organizational_unit()
            .next()
            .and_then(|v| v.to_string().ok())
            .ok_or(FromX509Error::OrganizationalUnitMissing { common_name })?;

        Ok(Self { name, id })
    }
}

pub fn find_development_teams() -> Result<Vec<Team>, Error> {
    let certs = {
        let new_name_scheme_cmd = get_pem_list_new_name_scheme();
        let new = new_name_scheme_cmd
            .run()
            .map_err(|error| Error::SecurityCommandFailed {
                command: format!("{new_name_scheme_cmd:?}"),
                error,
            })?;
        let mut certs =
            X509Certificate::from_pem_multiple(new.stdout).map_err(Error::X509ParseFailed)?;
        let old_name_scheme_cmd = get_pem_list_old_name_scheme();
        let old = old_name_scheme_cmd
            .run()
            .map_err(|error| Error::SecurityCommandFailed {
                command: format!("{old_name_scheme_cmd:?}"),
                error,
            })?;
        certs.append(
            &mut X509Certificate::from_pem_multiple(old.stdout).map_err(Error::X509ParseFailed)?,
        );
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
