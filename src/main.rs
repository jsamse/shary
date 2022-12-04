#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod common;
mod logging;
mod network;
mod ui;

use clap::Parser;
use color_eyre::Result;
use tracing::{event, Level};

use crate::common::AppState;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value_t = 17671)]
    port: u16,
}

fn main() -> Result<()> {
    logging::initialize()?;
    let args = Args::parse();
    event!(Level::INFO, ?args);

    let state = AppState::default();

    let _network = network::spawn(args.port, state.clone())?;

    ui::run(state);
    Ok(())
}
