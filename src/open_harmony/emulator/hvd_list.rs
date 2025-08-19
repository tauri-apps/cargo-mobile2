use super::Emulator;
use std::collections::BTreeSet;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to read HVD list: {0}")]
    ReadHvdFailed(std::io::Error),
    #[error("Failed to parse HVD list: {0}")]
    ParseHvdFailed(serde_json::Error),
}

pub fn hvd_list() -> Result<BTreeSet<Emulator>, Error> {
    let list_config_path = dirs::home_dir()
        .unwrap()
        .join(".Huawei/Emulator/deployed/lists.json");
    let hvd_json = std::fs::read_to_string(list_config_path).map_err(Error::ReadHvdFailed)?;
    serde_json::from_str(&hvd_json).map_err(Error::ParseHvdFailed)
}
