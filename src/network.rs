mod discovery;
mod server;
#[cfg(test)]
mod test;

use self::discovery::{run_discovery_receiver, run_discovery_sender};
use self::server::{run_file_server, run_file_download};
use crate::common::{LocalFile, RemoteFile};
use color_eyre::Result;
use color_eyre::eyre::Context;
use const_str::ip_addr;
use tokio::runtime::Runtime;
use std::net::Ipv4Addr;
use std::net::SocketAddrV4;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;

const IPV4_MULTICAST_ADDR: Ipv4Addr = ip_addr!(v4, "224.0.0.139");

pub fn spawn(port: u16) -> Result<NetworkHandle> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .wrap_err("failed to create tokio runtime")?;

    let network = Arc::new(Network::new(port));
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
    Ok(NetworkHandle { runtime, network })
}

pub struct NetworkHandle {
    runtime: Runtime,
    network: Arc<Network>,
}

impl NetworkHandle {
    pub fn remote_files(&self) -> Arc<Vec<RemoteFile>> {
        Arc::clone(&*self.network.remote_files.borrow())
    }

    pub fn set_local_files(&self, files: Arc<Vec<LocalFile>>) -> Result<()> {
        self.network.local_files.send(files).wrap_err("failed to send local files")
    }

    pub fn download(&self, file: &RemoteFile, path: PathBuf) {
        let addr = file.addr.clone();
        let filename = file.file.clone();
        let path = path.clone();
        tracing::info!("Download started: {} {}", addr, filename);
        self.runtime.spawn(async move {
            match run_file_download(&addr, &filename, path.as_path()).await {
                Ok(_) => tracing::info!("Download finished: {} {}", addr, filename),
                Err(err) => tracing::error!("Download failed: {} {} {}", addr, filename, err),
            }
        });
    }
}

struct Network {
    pub local_files: watch::Sender<Arc<Vec<LocalFile>>>,
    pub remote_files: watch::Receiver<Arc<Vec<RemoteFile>>>,
    port: u16,
    local_files_rx: watch::Receiver<Arc<Vec<LocalFile>>>,
    remote_files_tx: watch::Sender<Arc<Vec<RemoteFile>>>,
}

impl Network {
    fn new(port: u16) -> Network {
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

    async fn run(&self) -> Result<()> {
        let send_handle = run_discovery_sender(
            self.local_files_rx.clone(),
            SocketAddrV4::new(IPV4_MULTICAST_ADDR, self.port),
        );

        let recv_handle =
            run_discovery_receiver(&self.remote_files_tx, self.port, IPV4_MULTICAST_ADDR);

        let server_handle = run_file_server(self.port, self.local_files_rx.clone());

        tokio::try_join!(send_handle, recv_handle, server_handle)?;

        Ok(())
    }
}
