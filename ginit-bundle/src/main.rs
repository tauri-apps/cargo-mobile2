mod archive;
mod package;
mod plugin;

use self::{package::Package, plugin::Plugin};
use ginit_core::{
    bundle::manifest,
    config::empty,
    util::{self, cli},
};
use std::{
    env,
    fmt::{self, Display},
    io,
    path::PathBuf,
};
use structopt::StructOpt;

#[derive(Debug)]
enum Error {
    CurrentDirFailed(io::Error),
    PluginLoadFailed(manifest::Error),
    BundleFailed(package::Error),
    ArchiveFailed(archive::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CurrentDirFailed(err) => {
                write!(f, "Failed to get current working directory: {}", err)
            }
            Self::PluginLoadFailed(err) => write!(f, "Failed to read plugin manifest: {}", err),
            Self::BundleFailed(err) => write!(f, "Failed to create bundle: {}", err),
            Self::ArchiveFailed(err) => write!(f, "Failed to archive bundle: {}", err),
        }
    }
}

#[cli::main(env!("CARGO_PKG_NAME"))]
#[derive(Debug, StructOpt)]
#[structopt(
    author = env!("CARGO_PKG_AUTHORS"),
    about = env!("CARGO_PKG_DESCRIPTION"),
    settings = cli::GLOBAL_SETTINGS,
)]
struct Input {
    #[structopt(flatten)]
    flags: cli::GlobalFlags,
    #[structopt(
        short = "i",
        long = "manifest-root",
        about = "Specify manifest root instead of using the current directory"
    )]
    manifest_root: Option<PathBuf>,
    #[structopt(
        short = "o",
        long = "bundle-root",
        about = "Specify bundle root instead of using the manifest root"
    )]
    bundle_root: Option<PathBuf>,
    #[structopt(flatten)]
    profile: cli::Profile,
    #[structopt(long = "zip", about = "Zip up bundle, for your utmost convenience")]
    zip: bool,
}

impl cli::Exec for Input {
    type Config = empty::Config;
    type Error = Error;

    fn global_flags(&self) -> cli::GlobalFlags {
        self.flags
    }

    fn exec(
        self,
        _config: Option<Self::Config>,
        _wrapper: &util::TextWrapper,
    ) -> Result<(), Self::Error> {
        let Self {
            manifest_root,
            bundle_root,
            profile: cli::Profile { profile },
            zip,
            ..
        } = self;

        let manifest_root = match manifest_root {
            Some(manifest_root) => manifest_root,
            None => env::current_dir().map_err(Error::CurrentDirFailed)?,
        };
        let bundle_root = match bundle_root {
            Some(bundle_root) => bundle_root,
            None => manifest_root.join("bundles"),
        };

        log::info!("using manifest root {:?}", manifest_root);
        log::info!("using bundle root {:?}", bundle_root);

        let is_ginit = manifest_root.join("ginit").exists()
            && manifest_root.join("ginit-bundle").exists()
            && manifest_root.join("ginit-core").exists()
            && manifest_root.join("ginit-install").exists()
            && manifest_root.join("ginit-macros").exists()
            && manifest_root.join("ginit-os").exists();
        let package = if is_ginit {
            log::info!("detected that package is ginit");
            Package::Ginit
        } else {
            log::info!("detected that package is a plugin");
            let plugin = Plugin::load(&manifest_root).map_err(Error::PluginLoadFailed)?;
            Package::Plugin(plugin)
        };

        let bundle_path = package
            .bundle(&manifest_root, &bundle_root, profile)
            .map_err(Error::BundleFailed)?;
        println!("Created bundle successfully! {}", bundle_path.display());

        if zip {
            let zip_path =
                archive::zip(&bundle_root, &bundle_path).map_err(Error::ArchiveFailed)?;
            println!("Archived bundle successfully! {}", zip_path.display());
        }

        Ok(())
    }
}
