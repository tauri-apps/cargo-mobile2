use crate::util;
use ginit::{
    config::Umbrella,
    core::{
        cli::{Cli, CliInput},
        exports::into_result::IntoResult as _,
        ipc::Client,
        opts,
    },
    plugin::{Configured, Error, Plugin, Unconfigured},
};
use std::{
    collections::HashMap,
    path::Path,
    process::{Child, Command},
    rc::Rc,
};

#[derive(Debug)]
pub struct ProcHandle {
    inner: Child,
}

impl Drop for ProcHandle {
    fn drop(&mut self) {
        if let Err(err) = self.inner.wait() {
            eprintln!("{}", err);
        }
    }
}

impl From<Child> for ProcHandle {
    fn from(inner: Child) -> Self {
        Self { inner }
    }
}

#[derive(Debug)]
pub struct PluginData<State> {
    client: Rc<Client>,
    handle: ProcHandle,
    plugin: Plugin<State>,
    cli: Option<Cli>,
}

impl<State> PluginData<State> {
    pub fn shutdown(self) -> Result<(), Error> {
        self.plugin.shutdown(&self.client)
    }

    pub fn cli(&self) -> Option<&Cli> {
        self.cli.as_ref()
    }

    pub fn cli_info(&self) -> Option<util::CliInfo<'_>> {
        self.cli().map(|cli| util::CliInfo::new(&self.plugin, cli))
    }

    pub fn configure(self, config: &Umbrella) -> Result<PluginData<Configured>, Error> {
        let plugin = self.plugin.configure(&self.client, &config)?;
        Ok(PluginData {
            client: self.client,
            handle: self.handle,
            plugin,
            cli: self.cli,
        })
    }
}

impl PluginData<Configured> {
    pub fn exec(&self, input: CliInput, noise_level: opts::NoiseLevel) -> Result<(), Error> {
        self.plugin.exec(&self.client, input, noise_level)
    }
}

#[derive(Debug)]
struct Plugins<State> {
    inner: HashMap<String, PluginData<State>>,
}

impl<State> Drop for Plugins<State> {
    fn drop(&mut self) {
        for (_, plugin) in self.inner.drain() {
            if let Err(err) = plugin.shutdown() {
                eprintln!("{}", err);
            }
        }
    }
}

impl<State> Plugins<State> {
    fn new() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct PluginMap<State> {
    client: Rc<Client>,
    plugins: Plugins<State>,
}

impl<State> PluginMap<State> {
    pub fn client(&self) -> &Rc<Client> {
        &self.client
    }

    pub fn get(&self, name: &str) -> Option<&PluginData<State>> {
        self.plugins.inner.get(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Plugin<State>> + Clone {
        self.plugins.inner.values().map(|plugin| &plugin.plugin)
    }

    pub fn subcommands(&self) -> Vec<util::CliInfo<'_>> {
        self.plugins
            .inner
            .values()
            .filter_map(|plugin| plugin.cli_info())
            .collect()
    }

    pub fn configure(mut self, config: &Umbrella) -> PluginMap<Configured> {
        let plugins = Plugins {
            inner: self
                .plugins
                .inner
                .drain()
                .map(|(name, plugin)| (name, plugin.configure(&config).expect("dang")))
                .collect(),
        };
        PluginMap {
            client: self.client,
            plugins,
        }
    }
}

impl PluginMap<Unconfigured> {
    pub fn new() -> Self {
        Self {
            client: Client::new().expect("uh-oh").into(),
            plugins: Plugins::new(),
        }
    }

    pub fn load(&mut self, name: &str) {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join(&format!("../target/debug/ginit-{}", name));
        let handle = Command::new(path)
            .spawn()
            .into_result()
            .expect("darn")
            .into();
        std::thread::sleep_ms(100);
        let plugin = Plugin::connect(&self.client, "android").expect("uh-oh!");
        let cli = plugin.cli(&self.client).expect("uh-oh!!");
        self.plugins.inner.insert(
            name.to_owned(),
            PluginData {
                client: Rc::clone(&self.client),
                handle,
                plugin,
                cli,
            },
        );
    }
}
