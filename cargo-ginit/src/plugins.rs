use crate::util;
use ginit::{
    config::Umbrella,
    core::{
        cli::{Cli, CliInput},
        exports::{
            into_result::{command::CommandError, IntoResult as _},
            nng,
        },
        ipc::Client,
        opts,
    },
    plugin::{Configured, Error, Plugin, Unconfigured},
};
use std::{
    collections::HashMap,
    fmt::{self, Display},
    path::Path,
    process::{Child, Command},
};

#[derive(Debug)]
pub struct PluginData<State> {
    handle: Child,
    plugin: Plugin<State>,
    cli: Option<Cli>,
}

impl<State> PluginData<State> {
    pub fn cli(&self) -> Option<&Cli> {
        self.cli.as_ref()
    }

    pub fn cli_info(&self) -> Option<util::CliInfo<'_>> {
        self.cli().map(|cli| util::CliInfo::new(&self.plugin, cli))
    }

    pub fn configure(self, config: &Umbrella) -> Result<PluginData<Configured>, (Error, Self)> {
        match self.plugin.configure(&config) {
            Ok(plugin) => Ok(PluginData {
                handle: self.handle,
                plugin,
                cli: self.cli,
            }),
            Err((err, plugin)) => Err((
                err,
                PluginData {
                    handle: self.handle,
                    plugin,
                    cli: self.cli,
                },
            )),
        }
    }
}

impl PluginData<Configured> {
    pub fn exec(&self, input: CliInput, noise_level: opts::NoiseLevel) -> Result<(), Error> {
        self.plugin.exec(input, noise_level)
    }
}

#[derive(Debug)]
pub struct PluginGuard<State> {
    inner: Option<PluginData<State>>,
}

impl<State> Drop for PluginGuard<State> {
    fn drop(&mut self) {
        if let Some(mut inner) = self.inner.take() {
            if let Err(err) = inner.plugin.shutdown() {
                eprintln!("{}", err);
            }
            if let Err(err) = inner.handle.wait() {
                eprintln!("{}", err);
            }
        }
    }
}

impl<State> From<PluginData<State>> for PluginGuard<State> {
    fn from(plugin: PluginData<State>) -> Self {
        Self {
            inner: Some(plugin),
        }
    }
}

impl<State> PluginGuard<State> {
    pub fn unwrap_take(mut self) -> PluginData<State> {
        self.inner
            .take()
            .expect("Developer error: accessed an empty plugin guard")
    }

    pub fn unwrap_ref(&self) -> &PluginData<State> {
        self.inner
            .as_ref()
            .expect("Developer error: accessed an empty plugin guard")
    }
}

#[derive(Debug)]
pub struct PluginMap<State> {
    plugins: HashMap<String, PluginGuard<State>>,
}

impl<State> PluginMap<State> {
    pub fn get(&self, name: &str) -> Option<&PluginData<State>> {
        self.plugins.get(name).map(|guard| guard.unwrap_ref())
    }

    pub fn iter(&self) -> impl Iterator<Item = &Plugin<State>> + Clone {
        self.plugins
            .values()
            .map(|guard| &guard.unwrap_ref().plugin)
    }

    pub fn subcommands(&self) -> Vec<util::CliInfo<'_>> {
        self.plugins
            .values()
            .filter_map(|guard| guard.unwrap_ref().cli_info())
            .collect()
    }

    pub fn configure(mut self, config: &Umbrella) -> Result<PluginMap<Configured>, Error> {
        Ok(PluginMap {
            plugins: self
                .plugins
                .drain()
                .map(|(name, guard)| {
                    match guard.unwrap_take().configure(&config) {
                        Ok(plugin) => Ok((name, plugin.into())),
                        Err((err, plugin)) => {
                            PluginGuard::from(plugin); // shutdown plugin
                            Err(err)
                        }
                    }
                })
                .collect::<Result<_, _>>()?,
        })
    }
}

#[derive(Debug)]
pub enum LoadError {
    SpawnFailed(CommandError),
    ClientFailed(nng::Error),
    ConnectFailed(Error),
    CliFailed(Error),
}

impl Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SpawnFailed(err) => write!(f, "Failed to spawn plugin subprocess: {}", err),
            Self::ClientFailed(err) => write!(f, "Failed to initialize client: {}", err),
            Self::ConnectFailed(err) => write!(f, "Failed to initiate plugin connection: {}", err),
            Self::CliFailed(err) => write!(f, "Failed to request plugin CLI info: {}", err),
        }
    }
}

impl PluginMap<Unconfigured> {
    pub fn new() -> Self {
        Self {
            plugins: Default::default(),
        }
    }

    pub fn load(&mut self, name: &str) -> Result<(), LoadError> {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join(&format!("../target/debug/ginit-{}", name));
        let handle = Command::new(path)
            .spawn()
            .into_result()
            .map_err(LoadError::SpawnFailed)?
            .into();
        std::thread::sleep_ms(100);
        let client = Client::new().map_err(LoadError::ClientFailed)?;
        let plugin = Plugin::connect(client, name).map_err(LoadError::ConnectFailed)?;
        let cli = plugin.cli().map_err(LoadError::CliFailed)?;
        self.plugins.insert(
            name.to_owned(),
            PluginData {
                handle,
                plugin,
                cli,
            }
            .into(),
        );
        Ok(())
    }
}
