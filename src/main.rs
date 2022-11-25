#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod common;
mod logging;
mod network;
mod ui;

use clap::Parser;
use color_eyre::Result;
use std::path::PathBuf;
use tracing::{event, Level};

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value_t = 17671)]
    port: u16,
}

fn main() -> Result<()> {
    logging::initialize()?;
    let args = Args::parse();
    event!(Level::INFO, ?args);
    let network_handle = network::run(args.port);
    ui::run(network_handle);
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