use crate::{
    apple::{
        config::{Config, Metadata},
        device::{self, Device, RunError},
        rust_version_check,
        target::{
            ArchiveConfig, ArchiveError, BuildConfig, BuildError, CheckError, CompileLibError,
            ExportError, Target,
        },
        NAME,
    },
    config::{
        metadata::{self, Metadata as OmniMetadata},
        Config as OmniConfig, LoadOrGenError,
    },
    define_device_prompt,
    device::PromptError,
    env::{Env, Error as EnvError},
    opts, os,
    target::{call_for_targets_with_fallback, TargetInvalid, TargetTrait as _},
    util::{
        self,
        cli::{
            self, Exec, GlobalFlags, Report, Reportable, TextWrapper, VERSION_LONG, VERSION_SHORT,
        },
        prompt,
    },
};
use std::{collections::HashMap, ffi::OsStr, path::PathBuf};
use structopt::{clap::AppSettings, StructOpt};

#[derive(Debug, StructOpt)]
#[structopt(
    bin_name = cli::bin_name(NAME),
    version = VERSION_SHORT,
    long_version = VERSION_LONG.as_str(),
    global_settings = cli::GLOBAL_SETTINGS,
    settings = cli::SETTINGS,
)]
pub struct Input {
    #[structopt(flatten)]
    flags: GlobalFlags,
    #[structopt(subcommand)]
    command: Command,
}

impl Input {
    pub fn new(flags: GlobalFlags, command: Command) -> Self {
        Self { flags, command }
    }
}

fn macos_from_platform(platform: &str) -> bool {
    platform == "macOS"
}

fn profile_from_configuration(configuration: &str) -> opts::Profile {
    if configuration == "release" {
        opts::Profile::Release
    } else {
        opts::Profile::Debug
    }
}

#[derive(Clone, Debug, StructOpt)]
pub enum Command {
    #[structopt(name = "open", about = "Open project in Xcode")]
    Open,
    #[structopt(name = "check", about = "Checks if code compiles for target(s)")]
    Check {
        #[structopt(name = "targets", default_value = Target::DEFAULT_KEY, possible_values = &Target::name_list())]
        targets: Vec<String>,
    },
    #[structopt(name = "build", about = "Builds static libraries for target(s)")]
    Build {
        #[structopt(name = "targets", default_value = Target::DEFAULT_KEY, possible_values = &Target::name_list())]
        targets: Vec<String>,
        #[structopt(flatten)]
        profile: cli::Profile,
    },
    #[structopt(name = "archive", about = "Builds and archives for targets(s)")]
    Archive {
        #[structopt(long = "build-number")]
        build_number: Option<u32>,
        #[structopt(name = "targets", default_value = Target::DEFAULT_KEY, possible_values = &Target::name_list())]
        targets: Vec<String>,
        #[structopt(flatten)]
        profile: cli::Profile,
    },
    #[structopt(name = "run", about = "Deploys IPA to connected device")]
    Run {
        #[structopt(flatten)]
        profile: cli::Profile,
    },
    #[structopt(name = "list", about = "Lists connected devices")]
    List,
    #[structopt(name = "pod", about = "Runs `pod <args>`")]
    Pod {
        #[structopt(
            name = "arguments",
            help = "arguments passed down to the `pod <args>` command",
            index = 1,
            required = true
        )]
        arguments: Vec<String>,
    },
    #[structopt(
        name = "xcode-script",
        about = "Compiles static lib (should only be called by Xcode!)",
        setting = AppSettings::Hidden
    )]
    XcodeScript {
        #[structopt(
            long = "platform",
            help = "Value of `PLATFORM_DISPLAY_NAME` env var",
            parse(from_str = macos_from_platform),
        )]
        macos: bool,
        #[structopt(long = "sdk-root", help = "Value of `SDKROOT` env var")]
        sdk_root: PathBuf,
        #[structopt(
            long = "framework-search-paths",
            help = "Value of `FRAMEWORK_SEARCH_PATHS` env var"
        )]
        framework_search_paths: String,
        #[structopt(
            long = "gcc-preprocessor-definitions",
            help = "Value of `GCC_PREPROCESSOR_DEFINITIONS` env var"
        )]
        gcc_preprocessor_definitions: String,
        #[structopt(
            long = "header-search-paths",
            help = "Value of `HEADER_SEARCH_PATHS` env var"
        )]
        header_search_paths: String,
        #[structopt(
            long = "configuration",
            help = "Value of `CONFIGURATION` env var",
            parse(from_str = profile_from_configuration),
        )]
        profile: opts::Profile,
        #[structopt(long = "force-color", help = "Value of `FORCE_COLOR` env var")]
        force_color: bool,
        #[structopt(
            name = "ARCHS",
            help = "Value of `ARCHS` env var",
            index = 1,
            required = true
        )]
        arches: Vec<String>,
    },
}

#[derive(Debug)]
pub enum Error {
    EnvInitFailed(EnvError),
    RustVersionCheckFailed(util::RustVersionError),
    DevicePromptFailed(PromptError<String>),
    TargetInvalid(TargetInvalid),
    ConfigFailed(LoadOrGenError),
    MetadataFailed(metadata::Error),
    Unsupported,
    ProjectDirAbsent { project_dir: PathBuf },
    OpenFailed(os::OpenFileError),
    CheckFailed(CheckError),
    BuildFailed(BuildError),
    ArchiveFailed(ArchiveError),
    ExportFailed(ExportError),
    RunFailed(RunError),
    ListFailed(String),
    NoHomeDir(util::NoHomeDir),
    CargoEnvFailed(std::io::Error),
    SdkRootInvalid { sdk_root: PathBuf },
    IncludeDirInvalid { include_dir: PathBuf },
    MacosSdkRootInvalid { macos_sdk_root: PathBuf },
    ArchInvalid { arch: String },
    CompileLibFailed(CompileLibError),
    PodCommandFailed(std::io::Error),
    CopyLibraryFailed(std::io::Error),
    LibNotFound { path: PathBuf },
}

impl Reportable for Error {
    fn report(&self) -> Report {
        match self {
            Self::EnvInitFailed(err) => err.report(),
            Self::RustVersionCheckFailed(err) => err.report(),
            Self::DevicePromptFailed(err) => err.report(),
            Self::TargetInvalid(err) => Report::error("Specified target was invalid", err),
            Self::ConfigFailed(err) => err.report(),
            Self::MetadataFailed(err) => err.report(),
            Self::Unsupported => Report::error("iOS is marked as unsupported in your Cargo.toml metadata", "If your project should support Android, modify your Cargo.toml, then run `cargo mobile init` and try again."),
            Self::ProjectDirAbsent { project_dir } => Report::action_request(
                "Please run `cargo mobile init` and try again!",
                format!("Xcode project directory {:?} doesn't exist.", project_dir),
            ),
            Self::OpenFailed(err) => Report::error("Failed to open project in Xcode", err),
            Self::CheckFailed(err) => err.report(),
            Self::BuildFailed(err) => err.report(),
            Self::ArchiveFailed(err) => err.report(),
            Self::ExportFailed(err) => err.report(),
            Self::RunFailed(err) => err.report(),
            Self::ListFailed(err) => Report::error("Failed to list devices", err),
            Self::NoHomeDir(err) => Report::error("Failed to load cargo env profile", err),
            Self::CargoEnvFailed(err) => Report::error("Failed to load cargo env profile", err),
            Self::SdkRootInvalid { sdk_root } => Report::error(
                "SDK root provided by Xcode was invalid",
                format!("{:?} doesn't exist or isn't a directory", sdk_root),
            ),
            Self::IncludeDirInvalid { include_dir } => Report::error(
                "Include dir was invalid",
                format!("{:?} doesn't exist or isn't a directory", include_dir),
            ),
            Self::MacosSdkRootInvalid { macos_sdk_root } => Report::error(
                "macOS SDK root was invalid",
                format!("{:?} doesn't exist or isn't a directory", macos_sdk_root),
            ),
            Self::ArchInvalid { arch } => Report::error(
                "Arch specified by Xcode was invalid",
                format!("{:?} isn't a known arch", arch),
            ),
            Self::CompileLibFailed(err) => err.report(),
            Self::PodCommandFailed(err) => Report::error("pod command failed", err),
            Self::CopyLibraryFailed(err) => Report::error("Failed to copy static library to Xcode Project", err),
            Self::LibNotFound { path } => Report::error("Library artifact not found", format!("Library not found at {}. Make sure your Cargo.toml file has a [lib] block with `crate-type = [\"staticlib\", \"cdylib\", \"rlib\"]`", path.display())),
        }
    }
}

impl Exec for Input {
    type Report = Error;

    fn global_flags(&self) -> GlobalFlags {
        self.flags
    }

    fn exec(self, wrapper: &TextWrapper) -> Result<(), Self::Report> {
        define_device_prompt!(crate::apple::device::list_devices, String, iOS);
        fn detect_target_ok<'a>(env: &Env) -> Option<&'a Target<'a>> {
            device_prompt(env).map(|device| device.target()).ok()
        }

        fn with_config(
            non_interactive: bool,
            wrapper: &TextWrapper,
            f: impl FnOnce(&Config, &Metadata) -> Result<(), Error>,
        ) -> Result<(), Error> {
            let (config, _origin) = OmniConfig::load_or_gen(".", non_interactive, wrapper)
                .map_err(Error::ConfigFailed)?;
            let metadata =
                OmniMetadata::load(config.app().root_dir()).map_err(Error::MetadataFailed)?;
            if metadata.apple().supported() {
                f(config.apple(), metadata.apple())
            } else {
                Err(Error::Unsupported)
            }
        }

        fn ensure_init(config: &Config) -> Result<(), Error> {
            if !config.project_dir_exists() {
                Err(Error::ProjectDirAbsent {
                    project_dir: config.project_dir(),
                })
            } else {
                Ok(())
            }
        }

        fn open_in_xcode(config: &Config) -> Result<(), Error> {
            os::open_in_xcode(config.project_dir()).map_err(Error::OpenFailed)
        }

        let version_check = || rust_version_check(wrapper).map_err(Error::RustVersionCheckFailed);

        let Self {
            flags:
                GlobalFlags {
                    noise_level,
                    non_interactive,
                },
            command,
        } = self;
        let env = Env::new().map_err(Error::EnvInitFailed)?;
        match command {
            Command::Open => {
                version_check()?;
                with_config(non_interactive, wrapper, |config, _| {
                    ensure_init(config)?;
                    open_in_xcode(config)
                })
            }
            Command::Check { targets } => {
                version_check()?;
                with_config(non_interactive, wrapper, |config, metadata| {
                    call_for_targets_with_fallback(
                        targets.iter(),
                        &detect_target_ok,
                        &env,
                        |target: &Target| {
                            target
                                .check(config, metadata, &env, noise_level)
                                .map_err(Error::CheckFailed)
                        },
                    )
                    .map_err(Error::TargetInvalid)?
                })
            }
            Command::Build {
                targets,
                profile: cli::Profile { profile },
            } => with_config(non_interactive, wrapper, |config, _| {
                version_check()?;
                ensure_init(config)?;
                call_for_targets_with_fallback(
                    targets.iter(),
                    &detect_target_ok,
                    &env,
                    |target: &Target| {
                        target
                            .build(
                                config,
                                &env,
                                noise_level,
                                profile,
                                BuildConfig::default().allow_provisioning_updates(),
                            )
                            .map_err(Error::BuildFailed)
                    },
                )
                .map_err(Error::TargetInvalid)?
            }),
            Command::Archive {
                targets,
                build_number,
                profile: cli::Profile { profile },
            } => with_config(non_interactive, wrapper, |config, _| {
                version_check()?;
                ensure_init(config)?;
                call_for_targets_with_fallback(
                    targets.iter(),
                    &detect_target_ok,
                    &env,
                    |target: &Target| {
                        let mut app_version = config.bundle_version().clone();
                        if let Some(build_number) = build_number {
                            app_version.push_extra(build_number);
                        }

                        target
                            .build(
                                config,
                                &env,
                                noise_level,
                                profile,
                                BuildConfig::new().allow_provisioning_updates(),
                            )
                            .map_err(Error::BuildFailed)?;
                        target
                            .archive(
                                config,
                                &env,
                                noise_level,
                                profile,
                                Some(app_version),
                                ArchiveConfig::new().allow_provisioning_updates(),
                            )
                            .map_err(Error::ArchiveFailed)
                    },
                )
                .map_err(Error::TargetInvalid)?
            }),
            Command::Run {
                profile: cli::Profile { profile },
            } => with_config(non_interactive, wrapper, |config, _| {
                version_check()?;
                ensure_init(config)?;
                device_prompt(&env)
                    .map_err(Error::DevicePromptFailed)?
                    .run(config, &env, noise_level, non_interactive, profile)
                    .and_then(|h| {
                        h.wait()
                            .map(|_| ())
                            .map_err(|e| RunError::DeployFailed(e.to_string()))
                    })
                    .map_err(Error::RunFailed)
            }),
            Command::List => {
                device::list_devices(&env)
                    .map_err(Error::ListFailed)
                    .map(|device_list| {
                        prompt::list_display_only(device_list.iter(), device_list.len());
                    })
            }
            Command::Pod { mut arguments } => with_config(non_interactive, wrapper, |config, _| {
                arguments.push(format!(
                    "--project-directory={}",
                    config.project_dir().display()
                ));
                duct::cmd("pod", arguments)
                    .run()
                    .map_err(Error::PodCommandFailed)?;
                Ok(())
            }),
            Command::XcodeScript {
                macos,
                sdk_root,
                framework_search_paths,
                gcc_preprocessor_definitions,
                header_search_paths,
                profile,
                force_color,
                arches,
            } => with_config(non_interactive, wrapper, |config, metadata| {
                // The `PATH` env var Xcode gives us is missing any additions
                // made by the user's profile, so we'll manually add cargo's
                // `PATH`.
                let env = env.prepend_to_path(
                    util::home_dir()
                        .map_err(Error::NoHomeDir)?
                        .join(".cargo/bin"),
                );

                if !sdk_root.is_dir() {
                    return Err(Error::SdkRootInvalid { sdk_root });
                }
                let include_dir = sdk_root.join("usr/include");
                if !include_dir.is_dir() {
                    return Err(Error::IncludeDirInvalid { include_dir });
                }

                let mut host_env = HashMap::<&str, &OsStr>::new();

                // Host flags that are used by build scripts
                let (macos_isysroot, library_path) = {
                    let macos_sdk_root =
                        sdk_root.join("../../../../MacOSX.platform/Developer/SDKs/MacOSX.sdk");
                    if !macos_sdk_root.is_dir() {
                        return Err(Error::MacosSdkRootInvalid { macos_sdk_root });
                    }
                    (
                        format!("-isysroot {}", macos_sdk_root.display()),
                        format!("{}/usr/lib", macos_sdk_root.display()),
                    )
                };
                host_env.insert("MAC_FLAGS", macos_isysroot.as_ref());
                host_env.insert("CFLAGS_x86_64_apple_darwin", macos_isysroot.as_ref());
                host_env.insert("CXXFLAGS_x86_64_apple_darwin", macos_isysroot.as_ref());

                host_env.insert(
                    "OBJC_INCLUDE_PATH_x86_64_apple_darwin",
                    include_dir.as_os_str(),
                );

                host_env.insert("RUST_BACKTRACE", "1".as_ref());

                host_env.insert("FRAMEWORK_SEARCH_PATHS", framework_search_paths.as_ref());
                host_env.insert(
                    "GCC_PREPROCESSOR_DEFINITIONS",
                    gcc_preprocessor_definitions.as_ref(),
                );
                host_env.insert("HEADER_SEARCH_PATHS", header_search_paths.as_ref());

                let macos_target = Target::macos();

                let isysroot = format!("-isysroot {}", sdk_root.display());

                for arch in arches {
                    // FIXME Build the rust crate for iOS Simulator target too.
                    if arch == "Simulator" {
                        continue;
                    }

                    // Set target-specific flags
                    let (triple, rust_triple) = match arch.as_str() {
                        "arm64" => ("aarch64_apple_ios", "aarch64-apple-ios"),
                        // FIXME triple for cflags seems incorrect and we don't actually need to
                        // set it when cross compile simulator target.
                        // "arm64-sim" => ("aarch64_apple_ios", "aarch64-apple-ios"),
                        "x86_64" => ("x86_64_apple_ios", "x86_64-apple-ios"),
                        _ => return Err(Error::ArchInvalid { arch }),
                    };
                    let cflags = format!("CFLAGS_{}", triple);
                    let cxxflags = format!("CFLAGS_{}", triple);
                    let objc_include_path = format!("OBJC_INCLUDE_PATH_{}", triple);
                    let mut target_env = host_env.clone();
                    target_env.insert(cflags.as_ref(), isysroot.as_ref());
                    target_env.insert(cxxflags.as_ref(), isysroot.as_ref());
                    target_env.insert(objc_include_path.as_ref(), include_dir.as_ref());

                    let target = if macos {
                        // Prevents linker errors in build scripts and proc macros:
                        // https://github.com/signalapp/libsignal-client/commit/02899cac643a14b2ced7c058cc15a836a2165b6d
                        target_env.insert("LIBRARY_PATH", library_path.as_ref());
                        &macos_target
                    } else {
                        Target::for_arch(&arch).ok_or_else(|| Error::ArchInvalid {
                            arch: arch.to_owned(),
                        })?
                    };

                    target
                        .compile_lib(
                            config,
                            metadata,
                            noise_level,
                            force_color,
                            profile,
                            &env,
                            target_env,
                        )
                        .map_err(Error::CompileLibFailed)?;

                    let lib_location = format!(
                        "{rust_triple}/{}/lib{}.a",
                        profile.as_str(),
                        config.app().lib_name()
                    );
                    let lib_path = PathBuf::from(format!("../../target/{lib_location}"));

                    if !lib_path.exists() {
                        return Err(Error::LibNotFound { path: lib_path });
                    }

                    // Copy static lib .a to Xcode Project
                    if rust_triple == "aarch64-apple-ios" {
                        std::fs::create_dir_all(format!(
                            "Sources/{rust_triple}/{}",
                            profile.as_str()
                        ))
                        .map_err(Error::CopyLibraryFailed)?;

                        std::fs::copy(lib_path, format!("Sources/{lib_location}"))
                            .map_err(Error::CopyLibraryFailed)?;
                    }
                }
                Ok(())
            }),
        }
    }
}
