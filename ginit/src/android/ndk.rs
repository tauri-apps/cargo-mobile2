use std::{
    fs::File,
    io,
    num::ParseIntError,
    path::{Path, PathBuf},
};

const MIN_NDK_VERSION: u32 = 19;

#[cfg(target_os = "macos")]
pub fn host_tag() -> &'static str {
    "darwin-x86_64"
}

#[cfg(target_os = "linux")]
pub fn host_tag() -> &'static str {
    "linux-x86_64"
}

#[cfg(all(target_os = "window", target_pointer_width = "32"))]
pub fn host_tag() -> &'static str {
    "windows"
}

#[cfg(all(target_os = "window", target_pointer_width = "64"))]
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
            Compiler::Clang => "clang",
            Compiler::Clangxx => "clang++",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Binutil {
    Ar,
    Ld,
}

impl Binutil {
    fn as_str(&self) -> &'static str {
        match self {
            Binutil::Ar => "ar",
            Binutil::Ld => "ld",
        }
    }
}

#[derive(Debug)]
pub struct MissingToolError {
    name: &'static str,
    tried_path: PathBuf,
}

#[derive(Debug)]
pub enum VersionError {
    FailedToOpenSourceProps(io::Error),
    FailedToParseSourceProps(java_properties::PropertiesError),
    VersionMissingFromSourceProps,
    VersionComponentNotNumerical(ParseIntError),
    VersionHadTooFewComponents,
}

#[derive(Debug)]
pub enum EnvError {
    // link to docs/etc.
    NdkHomeNotSet(std::env::VarError),
    NdkHomeNotADir,
    FailedToGetVersion(VersionError),
    // "At least NDK r{} is required (you currently have NDK r{})"
    // the minor version could be used to get a b suffix, too! - should make a version struct
    VersionTooLow { you_have: u32, you_need: u32 },
}

pub struct Env {
    ndk_home: PathBuf,
}

impl Env {
    pub fn new() -> Result<Self, EnvError> {
        let ndk_home = std::env::var("NDK_HOME")
            .map_err(EnvError::NdkHomeNotSet)
            .map(PathBuf::from)
            .and_then(|ndk_home| {
                if ndk_home.is_dir() {
                    Ok(ndk_home)
                } else {
                    Err(EnvError::NdkHomeNotADir)
                }
            })?;
        let env = Self { ndk_home };
        let (major, ..) = env.version().map_err(EnvError::FailedToGetVersion)?;
        if major >= MIN_NDK_VERSION {
            Ok(env)
        } else {
            Err(EnvError::VersionTooLow {
                you_have: major,
                you_need: MIN_NDK_VERSION,
            })
        }
    }

    pub fn home(&self) -> &Path {
        &self.ndk_home
    }

    pub fn version(&self) -> Result<(u32, u32), VersionError> {
        let file = File::open(self.ndk_home.join("source.properties"))
            .map_err(VersionError::FailedToOpenSourceProps)?;
        let props = java_properties::read(file).map_err(VersionError::FailedToParseSourceProps)?;
        let revision = props
            .get("Pkg.Revision")
            .ok_or(VersionError::VersionMissingFromSourceProps)?;
        // The possible revision formats can be found in the comments of
        // `$NDK_HOME/build/cmake/android.toolchain.cmake` - only the last component
        // can be non-numerical, which we're not using anyway. If that changes,
        // then the aforementioned file contains a regex we can use.
        let components = revision
            .split('.')
            .take(2)
            .map(|component| component.parse::<u32>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(VersionError::VersionComponentNotNumerical)?;
        if components.len() == 2 {
            Ok((components[0], components[1]))
        } else {
            Err(VersionError::VersionHadTooFewComponents)
        }
    }

    pub fn tool_dir(&self) -> Result<PathBuf, MissingToolError> {
        let path = self
            .ndk_home
            .join(format!("toolchains/llvm/prebuilt/{}/bin", host_tag()));
        if path.is_dir() {
            Ok(path)
        } else {
            // TODO: this might be too silly
            Err(MissingToolError {
                name: "literally all of them",
                tried_path: path,
            })
        }
    }

    pub fn compiler_path(
        &self,
        compiler: Compiler,
        triple: &str,
        min_api: u32,
    ) -> Result<PathBuf, MissingToolError> {
        let path = self
            .tool_dir()?
            .join(format!("{}{:?}-{}", triple, min_api, compiler.as_str()));
        if path.is_file() {
            Ok(path)
        } else {
            Err(MissingToolError {
                name: compiler.as_str(),
                tried_path: path,
            })
        }
    }

    pub fn binutil_path(
        &self,
        binutil: Binutil,
        triple: &str,
    ) -> Result<PathBuf, MissingToolError> {
        let path = self
            .tool_dir()?
            .join(format!("{}-{}", triple, binutil.as_str()));
        if path.is_file() {
            Ok(path)
        } else {
            Err(MissingToolError {
                name: binutil.as_str(),
                tried_path: path,
            })
        }
    }
}
