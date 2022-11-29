#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod common;
mod logging;
mod network;
mod ui;

use std::{sync::Arc, time::Duration};

use clap::Parser;
use color_eyre::{eyre::WrapErr, Result};
use tracing::{event, Level};

use crate::network::Network;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value_t = 17671)]
    port: u16,
}

fn main() -> Result<()> {
    logging::initialize()?;
    let args = Args::parse();
    event!(Level::INFO, ?args);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .wrap_err("failed to create tokio runtime")?;

    let network = Arc::new(Network::new(args.port));
    {
        let network = Arc::clone(&network);
        runtime.spawn(async move {
            loop {
                let result = network.run().await;
                tracing::error!("Network completed with result: {:?}", result);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });
    }

    ui::run(network);
    Ok(())
}
