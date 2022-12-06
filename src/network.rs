mod discovery;
mod server;
#[cfg(test)]
mod test;

use self::discovery::{run_discovery_receiver, run_discovery_sender};
use self::server::{run_file_server, run_file_download};
use crate::common::Files;
use color_eyre::Result;
use color_eyre::eyre::Context;
use const_str::ip_addr;
use tokio::runtime::Runtime;
use std::net::Ipv4Addr;
use std::net::SocketAddrV4;
use std::sync::Arc;
use std::time::Duration;

const IPV4_MULTICAST_ADDR: Ipv4Addr = ip_addr!(v4, "224.0.0.139");

pub fn spawn(port: u16, files: Arc<Files>) -> Result<NetworkHandle> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .wrap_err("failed to create tokio runtime")?;

    let network = Arc::new(Network::new(port, files));
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
    Ok(NetworkHandle { _runtime: runtime, _network: network })
}

pub struct NetworkHandle {
    _runtime: Runtime,
    _network: Arc<Network>,
}

struct Network {
    port: u16,
    files: Arc<Files>,
}

impl Network {
    fn new(port: u16, files: Arc<Files>) -> Network {
        Network {
            port,
            files,
        }
    }

    async fn run(&self) -> Result<()> {
        let send_handle = run_discovery_sender(
            self.files.get_local_files(),
            SocketAddrV4::new(IPV4_MULTICAST_ADDR, self.port),
        );

        let recv_handle =
            run_discovery_receiver(&self.files.remote_files_tx, self.port, IPV4_MULTICAST_ADDR);

        let server_handle = run_file_server(self.port, self.files.get_local_files());

        let download_handle = run_file_download(&self.files);

        tokio::try_join!(send_handle, recv_handle, server_handle, download_handle)?;

        Ok(())
    }
}
