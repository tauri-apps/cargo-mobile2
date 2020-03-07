use ginit_core::{
    exports::toml,
    storage::{global_config::GlobalConfig, Storage},
};
use std::fs;

fn main() {
    let storage = Storage::new().expect("failed to get storage path");
    fs::create_dir_all(storage.path()).expect("failed to create storage dir");
    let path = storage.global_config_path();
    let ser = toml::to_string_pretty(&GlobalConfig {
        default_plugins: vec![
            "brainium".to_owned(),
            "android".to_owned(),
            "ios".to_owned(),
        ],
    })
    .expect("failed to serialize global config");
    fs::write(path, ser).expect("failed to write global config");
}
