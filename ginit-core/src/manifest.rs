use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub name: String,
    pub description: String,
    pub supports: Vec<String>,
}
