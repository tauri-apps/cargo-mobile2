use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
    sync::Mutex,
};

fn file_name() -> String {
    format!("{}.toml", crate::NAME)
}

fn check_path() -> Option<PathBuf> {
    Config::discover_root().expect("Failed to canonicalize current directory")
}

lazy_static::lazy_static! {
    static ref MAYBE_ROOT: Mutex<Option<PathBuf>> = Mutex::new(check_path());
    static ref PROJECT_ROOT: PathBuf = MAYBE_ROOT
        .lock()
        .unwrap()
        .clone()
        .expect("Failed to find config file");
    pub static ref CONFIG: Config = Config::load();
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub global: Global,
    pub android: crate::android::Config,
    pub ios: crate::ios::Config,
}

impl Config {
    fn discover_root() -> io::Result<Option<PathBuf>> {
        let mut path = Path::new(".").canonicalize()?.join(&file_name());
        while !path.exists() {
            if let Some(parent) = path.parent().and_then(Path::parent) {
                path = parent.join(&file_name());
                log::info!("Looking for config file at {:?}", path);
            } else {
                return Ok(None);
            }
        }
        log::info!("Found config file at {:?}", path);
        path.pop();
        Ok(Some(path))
    }

    pub fn exists() -> bool {
        MAYBE_ROOT.lock().unwrap().is_some()
    }

    pub fn recheck_path() {
        *MAYBE_ROOT.lock().unwrap() = check_path();
    }

    fn load() -> Self {
        let path = PROJECT_ROOT.join(&file_name());
        let mut file = File::open(&path).expect("Failed to open config file");
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .expect("Failed to read config file");
        toml::from_slice(&contents).expect("Failed to parse config file")
    }

    pub fn project_root(&self) -> &'static Path {
        &PROJECT_ROOT
    }

    pub fn prefix_path(&self, path: impl AsRef<Path>) -> PathBuf {
        PROJECT_ROOT.join(path)
    }

    pub fn unprefix_path(&self, path: impl AsRef<Path>) -> PathBuf {
        path.as_ref()
            .strip_prefix(&*PROJECT_ROOT)
            .expect("`unprefix_path` called on path that wasn't prefixed")
            .to_owned()
    }

    pub fn app_name(&self) -> &str {
        &self.global.app_name
    }

    pub fn stylized_app_name(&self) -> &str {
        self.global
            .stylized_app_name
            .as_ref()
            .unwrap_or_else(|| &self.global.app_name)
    }

    pub fn reverse_domain(&self) -> String {
        self.global
            .domain
            .clone()
            .split('.')
            .rev()
            .collect::<Vec<_>>()
            .join(".")
    }

    pub fn source_root(&self) -> PathBuf {
        self.prefix_path(&self.global.source_root)
    }

    pub fn manifest_path(&self) -> Option<PathBuf> {
        self.global
            .manifest_path
            .as_ref()
            .map(|path| self.prefix_path(path))
    }

    pub fn asset_path(&self) -> PathBuf {
        self.prefix_path(&self.global.asset_path)
    }

    pub fn insert_data(&self, map: &mut bicycle::JsonMap) {
        map.insert("config", &self);
        map.insert("app_name", self.app_name());
        map.insert("stylized_app_name", self.stylized_app_name());
        map.insert("reverse_domain", self.reverse_domain());
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Global {
    pub app_name: String,
    pub stylized_app_name: Option<String>,
    pub domain: String,
    pub source_root: String,
    pub manifest_path: Option<String>,
    pub asset_path: String,
}
