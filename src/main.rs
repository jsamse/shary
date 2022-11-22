#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod common;
mod logging;
mod network;
mod ui;

use clap::Parser;
use color_eyre::Result;
use std::path::PathBuf;
use tracing::{event, field, info, instrument, Level};

use crate::common::Key;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value_t = 17671)]
    discovery_port: u16,
    #[arg(short, long, default_value_t = 17672)]
    transfer_port: u16,
}

fn main() -> Result<()> {
    logging::initialize()?;
    let args = Args::parse();
    event!(Level::INFO, ?args);
    let key = Key::new();
    ui::run(key, args.discovery_port);
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
