use std::io::Write;

use color_eyre::{Result, eyre::Context};

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

    pub fn write_all_to<T: Write>(&self, w: &mut T) -> Result<()> {
        w.write_all(self.as_str().as_bytes()).wrap_err("failed to write_all")
    }
}