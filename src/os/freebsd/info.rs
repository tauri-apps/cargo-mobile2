use crate::os::Info;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to run `uname {flag}`: {source}")]
    UnameFailed { flag: &'static str, source: std::io::Error },
    #[error("`uname {flag}` returned empty output")]
    UnameEmpty { flag: &'static str },
}

pub fn check() -> Result<Info, Error> {
    let name = duct::cmd("uname", ["-s"])
        .read()
        .map_err(|source| Error::UnameFailed { flag: "-s", source })?;
    let name = name.trim().to_owned();
    if name.is_empty() {
        return Err(Error::UnameEmpty { flag: "-s" });
    }

    let version = duct::cmd("uname", ["-r"])
        .read()
        .map_err(|source| Error::UnameFailed { flag: "-r", source })?;
    let version = version.trim().to_owned();
    if version.is_empty() {
        return Err(Error::UnameEmpty { flag: "-r" });
    }

    Ok(Info { name, version })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "freebsd")]
    fn check_returns_freebsd_info() {
        let info = check().expect("uname should succeed on FreeBSD");
        assert_eq!(info.name, "FreeBSD");
        assert!(!info.version.is_empty());
    }
}
