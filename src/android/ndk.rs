use super::{
    source_props::{self, SourceProps},
    target::Target,
};
use crate::{
    os::consts,
    util::{
        cli::{Report, Reportable},
        VersionDouble,
    },
};
use once_cell_regex::regex_multi_line;
use std::{
    collections::HashSet,
    fmt::{self, Display},
    path::{Path, PathBuf},
};
use thiserror::Error;

const MIN_NDK_VERSION: NdkVersion = NdkVersion(VersionDouble::new(19, 0));

#[cfg(target_os = "macos")]
pub fn host_tag() -> &'static str {
    "darwin-x86_64"
}

#[cfg(target_os = "linux")]
pub fn host_tag() -> &'static str {
    "linux-x86_64"
}

#[cfg(all(windows, target_pointer_width = "32"))]
pub fn host_tag() -> &'static str {
    "windows"
}

#[cfg(all(windows, target_pointer_width = "64"))]
pub fn host_tag() -> &'static str {
    "windows-x86_64"
}

#[derive(Clone, Copy, Debug)]
pub enum Compiler {
    Clang,
    Clangxx,
}

impl Compiler {
    fn as_str(&self) -> &'static str {
        match self {
            Compiler::Clang => consts::CLANG,
            Compiler::Clangxx => consts::CLANGXX,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Binutil {
    #[allow(dead_code)]
    Ld,
}

impl Binutil {
    fn as_str(&self) -> &'static str {
        match self {
            Binutil::Ld => consts::LD,
        }
    }
}

#[derive(Debug, Error)]
#[error("Missing tool `{name}`; tried at {tried_path:?}.")]
pub struct MissingToolError {
    name: &'static str,
    tried_path: PathBuf,
}

impl MissingToolError {
    fn check_file(path: PathBuf, name: &'static str) -> Result<PathBuf, Self> {
        if path.is_file() {
            Ok(path)
        } else {
            Err(Self {
                name,
                tried_path: path,
            })
        }
    }

    fn check_dir(path: PathBuf, name: &'static str) -> Result<PathBuf, Self> {
        if path.is_dir() {
            Ok(path)
        } else {
            Err(Self {
                name,
                tried_path: path,
            })
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct NdkVersion(VersionDouble);

impl Display for NdkVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "r{}", self.0.major)?;
        if self.0.minor != 0 {
            write!(
                f,
                "{}",
                (b'a'..=b'z')
                    .map(char::from)
                    .nth(self.0.minor as _)
                    .expect("NDK minor version exceeded the number of letters in the alphabet")
            )?;
        }
        Ok(())
    }
}

impl From<source_props::Revision> for NdkVersion {
    fn from(revision: source_props::Revision) -> Self {
        Self(VersionDouble::new(
            revision.triple.major,
            revision.triple.minor,
        ))
    }
}

#[derive(Debug, Error)]
pub enum Error {
    // TODO: link to docs/etc.
    #[error("Have you installed the NDK? The `NDK_HOME` environment variable isn't set, and is required: {0}")]
    NdkHomeNotSet(#[from] std::env::VarError),
    #[error("Have you installed the NDK? The `NDK_HOME` environment variable is set, but doesn't point to an existing directory.")]
    NdkHomeNotADir,
    #[error("Failed to lookup version of installed NDK: {0}")]
    VersionLookupFailed(#[from] source_props::Error),
    #[error("At least NDK {you_need} is required (you currently have NDK {you_have})")]
    VersionTooLow {
        you_have: NdkVersion,
        you_need: NdkVersion,
    },
}

impl Reportable for Error {
    fn report(&self) -> Report {
        Report::error("Failed to initialize NDK environment", self)
    }
}

#[derive(Debug, Error)]
pub enum RequiredLibsError {
    #[error(transparent)]
    MissingTool(#[from] MissingToolError),
    #[error(transparent)]
    ReadElfFailed(#[from] std::io::Error),
    #[error("`readelf` output contained invalid UTF-8: {0}")]
    InvalidUtf8(#[from] std::str::Utf8Error),
}

impl Reportable for RequiredLibsError {
    fn report(&self) -> Report {
        Report::error("Failed to get list of required libs", self)
    }
}

#[derive(Debug, Clone)]
pub struct Env {
    ndk_home: PathBuf,
}

impl Env {
    pub fn new() -> Result<Self, Error> {
        let ndk_home = std::env::var("NDK_HOME")
            .map_err(Error::NdkHomeNotSet)
            .map(PathBuf::from)
            .and_then(|ndk_home| {
                if ndk_home.is_dir() {
                    Ok(ndk_home)
                } else {
                    Err(Error::NdkHomeNotADir)
                }
            })?;
        let env = Self { ndk_home };
        let version = env
            .version()
            .map(NdkVersion::from)
            .map_err(Error::VersionLookupFailed)?;
        if version >= MIN_NDK_VERSION {
            Ok(env)
        } else {
            Err(Error::VersionTooLow {
                you_have: version,
                you_need: MIN_NDK_VERSION,
            })
        }
    }

    pub fn home(&self) -> &Path {
        &self.ndk_home
    }

    pub fn version(&self) -> Result<source_props::Revision, source_props::Error> {
        SourceProps::from_path(self.ndk_home.join("source.properties"))
            .map(|props| props.pkg.revision)
    }

    pub fn prebuilt_dir(&self) -> Result<PathBuf, MissingToolError> {
        MissingToolError::check_dir(
            self.ndk_home
                .join(format!("toolchains/llvm/prebuilt/{}", host_tag())),
            // TODO: shove this square peg into a squarer hole
            "prebuilt toolchain",
        )
    }

    pub fn tool_dir(&self) -> Result<PathBuf, MissingToolError> {
        MissingToolError::check_dir(self.prebuilt_dir()?.join("bin"), "tools")
    }

    pub fn compiler_path(
        &self,
        compiler: Compiler,
        triple: &str,
        min_api: u32,
    ) -> Result<PathBuf, MissingToolError> {
        MissingToolError::check_file(
            self.tool_dir()?
                .join(format!("{}{}-{}", triple, min_api, compiler.as_str())),
            compiler.as_str(),
        )
    }

    pub fn binutil_path(
        &self,
        binutil: Binutil,
        triple: &str,
    ) -> Result<PathBuf, MissingToolError> {
        MissingToolError::check_file(
            self.tool_dir()?
                .join(format!("{}-{}", triple, binutil.as_str())),
            binutil.as_str(),
        )
    }

    pub fn libcxx_shared_path(&self, target: Target<'_>) -> Result<PathBuf, MissingToolError> {
        static LIB: &str = "libc++_shared.so";
        let ndk_ver = self.version().unwrap_or_default();
        let so_path = if ndk_ver.triple.major >= 22 {
            let ndk_triple = if target.triple == "armv7-linux-androideabi" {
                "arm-linux-androideabi"
            } else {
                target.triple
            };
            self.prebuilt_dir()?
                .join("sysroot/usr/lib")
                .join(ndk_triple)
        } else {
            self.ndk_home
                .join("sources/cxx-stl/llvm-libc++/libs")
                .join(target.abi)
        };
        MissingToolError::check_file(so_path.join(LIB), LIB)
    }

    pub fn ar_path(&self, triple: &str) -> Result<PathBuf, MissingToolError> {
        let ndk_ver = self.version().unwrap_or_default();
        let bin_path = if ndk_ver.triple.major >= 23 {
            format!("llvm-{}", consts::AR)
        } else {
            format!("{}-{}", triple, consts::AR)
        };
        MissingToolError::check_file(self.tool_dir()?.join(bin_path), "ar")
    }

    fn readelf_path(&self, triple: &str) -> Result<PathBuf, MissingToolError> {
        let ndk_ver = self.version().unwrap_or_default();
        let bin_path = if ndk_ver.triple.major >= 23 {
            format!("llvm-{}", consts::READELF)
        } else {
            format!("{}-{}", triple, consts::READELF)
        };
        MissingToolError::check_file(self.tool_dir()?.join(bin_path), "readelf")
    }

    pub fn required_libs(
        &self,
        elf: &Path,
        triple: &str,
    ) -> Result<HashSet<String>, RequiredLibsError> {
        let elf_path = dunce::simplified(elf).to_owned();
        Ok(regex_multi_line!(r"\(NEEDED\)\s+Shared library: \[(.+)\]")
            .captures_iter(
                duct::cmd(self.readelf_path(triple)?, ["-d"])
                    .before_spawn(move |cmd| {
                        cmd.arg(&elf_path);
                        Ok(())
                    })
                    .stderr_capture()
                    .read()?
                    .as_str(),
            )
            .map(|caps| {
                let lib = caps
                    .get(1)
                    .expect("developer error: regex match had no captures")
                    .as_str();
                log::info!("{:?} requires shared lib {:?}", elf, lib);
                lib.to_owned()
            })
            .collect())
    }
}
