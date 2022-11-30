mod discovery;
mod server;
mod test;

use self::discovery::{run_discovery_receiver, run_discovery_sender};
use self::server::run_file_server;
use crate::common::{LocalFile, RemoteFile};
use color_eyre::Result;
use color_eyre::eyre::Context;
use const_str::ip_addr;
use std::net::Ipv4Addr;
use std::net::SocketAddrV4;
use std::sync::Arc;
use tokio::sync::watch;

const IPV4_MULTICAST_ADDR: Ipv4Addr = ip_addr!(v4, "224.0.0.139");

pub struct Network {
    pub local_files: watch::Sender<Arc<Vec<LocalFile>>>,
    pub remote_files: watch::Receiver<Arc<Vec<RemoteFile>>>,
    port: u16,
    local_files_rx: watch::Receiver<Arc<Vec<LocalFile>>>,
    remote_files_tx: watch::Sender<Arc<Vec<RemoteFile>>>,
}

impl Network {
    pub fn new(port: u16) -> Network {
        let (local_files_tx, local_files_rx) = watch::channel(Arc::new(vec![]));
        let (remote_files_tx, remote_files_rx) = watch::channel(Arc::new(vec![]));
        Network {
            local_files: local_files_tx,
            remote_files: remote_files_rx,
            port,
            local_files_rx,
            remote_files_tx,
        }
    }

    pub async fn run(&self) -> Result<()> {
        let send_handle = run_discovery_sender(
            self.local_files_rx.clone(),
            SocketAddrV4::new(IPV4_MULTICAST_ADDR, self.port),
        );

        let recv_handle =
            run_discovery_receiver(&self.remote_files_tx, self.port, IPV4_MULTICAST_ADDR);

        let server_handle = {
            let port = self.port;
            let local_files = self.local_files_rx.clone();
            async move {
                tokio::task::spawn_blocking(move || {
                    run_file_server(port, local_files)
                }).await.wrap_err("failed to run file server blocking")?
            }
        };

        tokio::try_join!(send_handle, recv_handle, server_handle)?;

        Ok(())
    }
}
