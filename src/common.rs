use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct Key {
    raw: String,
}

impl Key {
    pub fn new() -> Key {
        let raw = random_string::generate(6, "ABCDEFGHIJKLMNOPQRSTUVXYZ");
        Key { raw }
    }

    pub fn as_str(&self) -> &str {
        self.raw.as_ref()
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct LocalFile {
    path: PathBuf,
    name: String,
}
