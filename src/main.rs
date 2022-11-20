#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod logging;
mod ui;

use color_eyre::{eyre::Context, Report, Result};
use egui::Ui;
use rfd::FileDialog;
use std::{net::UdpSocket, path::PathBuf};
use tracing::info;

fn main() -> Result<()> {
    logging::initialize()?;
    let key = random_string::generate(6, "ABCDEFGHIJKLMNOPQRSTUVXYZ");
    ui::run(key);
    Ok(())
}

struct Send {
    path: PathBuf,
}

impl Send {
    fn name(&self) -> &str {
        return self.path.to_str().unwrap_or_default();
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn sends_equal() {
        let mut v = vec![];
        add_to_vec(&mut v);
        add_to_vec(&mut v);

        assert_eq!(0, v[0]);
        assert_eq!(1, v[1]);
    }

    fn add_to_vec(v: &mut Vec<usize>) {
        v.push(v.len());
    }
}
