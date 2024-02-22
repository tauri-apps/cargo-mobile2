use std::{
    borrow::Cow,
    fmt::{self, Display},
    fs::remove_dir_all,
    path::{Path, PathBuf},
};

use crate::DuctExpressionExt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinkType {
    Hard,
    Symbolic,
}

impl Display for LinkType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hard => write!(f, "hard"),
            Self::Symbolic => write!(f, "symbolic"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Clobber {
    Never,
    FileOnly,
    FileOrDirectory,
}

impl Display for Clobber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Never => write!(f, "clobbering disabled"),
            Self::FileOnly => write!(f, "file clobbering enabled"),
            Self::FileOrDirectory => write!(f, "file and directory clobbering enabled"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TargetStyle {
    File,
    Directory,
}

impl Display for TargetStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::File => write!(f, "file"),
            Self::Directory => write!(f, "directory"),
        }
    }
}

#[derive(Debug)]
pub enum ErrorCause {
    MissingFileName,
    CommandFailed(std::io::Error),
    IOError(std::io::Error),
    SymlinkNotAllowed,
}

impl Display for ErrorCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingFileName => {
                write!(f, "Neither the source nor target contained a file name.",)
            }
            Self::CommandFailed(err) => write!(f, "`ln` command failed: {}", err),
            Self::IOError(err) => write!(f, "IO error: {}", err),
            Self::SymlinkNotAllowed => {
                write!(
                    f,
                    r"
Creation symbolic link is not allowed for this system.

For Windows 10 or newer:
You should use developer mode.
See https://docs.microsoft.com/en-us/windows/apps/get-started/enable-your-device-for-development

For Window 8.1 or older:
You need `SeCreateSymbolicLinkPrivilege` security policy.
See https://docs.microsoft.com/en-us/windows/security/threat-protection/security-policy-settings/create-symbolic-links"
                )
            }
        }
    }
}

#[derive(Debug)]
pub struct Error {
    link_type: LinkType,
    force: Clobber,
    source: PathBuf,
    target: PathBuf,
    target_style: TargetStyle,
    cause: ErrorCause,
}

impl Error {
    pub fn new(
        link_type: LinkType,
        force: Clobber,
        source: PathBuf,
        target: PathBuf,
        target_style: TargetStyle,
        cause: ErrorCause,
    ) -> Self {
        Self {
            link_type,
            force,
            source,
            target,
            target_style,
            cause,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Failed to create a {} link from {:?} to {} {:?} ({}): {}",
            self.link_type, self.source, self.target_style, self.target, self.force, self.cause
        )
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Debug)]
pub struct Call<'a> {
    link_type: LinkType,
    force: Clobber,
    source: &'a Path,
    target: &'a Path,
    target_override: Cow<'a, Path>,
    target_style: TargetStyle,
}

impl<'a> Call<'a> {
    pub fn new(
        link_type: LinkType,
        force: Clobber,
        source: &'a Path,
        target: &'a Path,
        target_style: TargetStyle,
    ) -> Result<Self, Error> {
        let target_override = if let TargetStyle::Directory = target_style {
            // If the target is a directory, then the link name has to come from
            // the last component of the source.
            if let Some(file_name) = source.file_name() {
                Cow::Owned(target.join(file_name))
            } else {
                return Err(Error {
                    link_type,
                    force,
                    source: source.to_owned(),
                    target: target.to_owned(),
                    target_style,
                    cause: ErrorCause::MissingFileName,
                });
            }
        } else {
            Cow::Borrowed(target)
        };
        Ok(Self {
            link_type,
            force,
            source,
            target,
            target_override,
            target_style,
        })
    }

    pub fn exec(self) -> Result<(), Error> {
        let mut args = vec!["-n" /* don't follow symlinks */];
        if let LinkType::Symbolic = self.link_type {
            args.push("-s");
        }
        match self.force {
            Clobber::FileOnly => {
                args.push("-f");
            }
            Clobber::FileOrDirectory => {
                if self.target_override.is_dir() {
                    remove_dir_all(self.target)
                        .map_err(|err| self.make_error(ErrorCause::IOError(err)))?;
                }
                args.push("-f");
            }
            _ => (),
        }
        let source = self.source.to_string_lossy();
        let target_override = self.target_override.as_ref().to_string_lossy();
        args.push(&source);
        args.push(&target_override);
        duct::cmd("ln", args)
            .dup_stdio()
            .run()
            .map_err(|err| self.make_error(ErrorCause::CommandFailed(err)))?;
        Ok(())
    }

    fn make_error(&self, cause: ErrorCause) -> Error {
        Error {
            link_type: self.link_type,
            force: self.force,
            source: self.source.to_owned(),
            target: self.target.to_owned(),
            target_style: self.target_style,
            cause,
        }
    }
}

pub fn force_symlink(
    source: impl AsRef<Path>,
    target: impl AsRef<Path>,
    target_style: TargetStyle,
) -> Result<(), Error> {
    Call::new(
        LinkType::Symbolic,
        Clobber::FileOrDirectory,
        source.as_ref(),
        target.as_ref(),
        target_style,
    )?
    .exec()
}

pub fn force_symlink_relative(
    abs_source: impl AsRef<Path>,
    abs_target: impl AsRef<Path>,
    target_style: TargetStyle,
) -> Result<(), Error> {
    let (abs_source, abs_target) = (abs_source.as_ref(), abs_target.as_ref());
    let rel_source = super::relativize_path(abs_source, abs_target);
    if target_style == TargetStyle::Directory && rel_source.file_name().is_none() {
        if let Some(file_name) = abs_source.file_name() {
            force_symlink(rel_source, abs_target.join(file_name), TargetStyle::File)
        } else {
            Err(Error {
                link_type: LinkType::Symbolic,
                force: Clobber::FileOrDirectory,
                source: rel_source,
                target: abs_target.to_owned(),
                target_style,
                cause: ErrorCause::MissingFileName,
            })
        }
    } else {
        force_symlink(rel_source, abs_target, target_style)
    }
}
