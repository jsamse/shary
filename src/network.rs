mod discovery;
mod server;
mod test;

use self::discovery::{spawn_discovery_receiver, spawn_discovery_sender};
use crate::common::{LocalFile, RemoteFile};
use color_eyre::Result;
use const_str::ip_addr;
use std::collections::HashMap;
use std::net::SocketAddrV4;
use std::sync::Arc;
use std::{
    net::Ipv4Addr,
    path::{Path, PathBuf},
};
use tokio::net::{TcpListener, UdpSocket};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tracing::error;

const IPV4_MULTICAST_ADDR: Ipv4Addr = ip_addr!(v4, "224.0.0.139");

pub fn run(port: u16) -> NetworkHandle {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to create tokio runtime");

    let (status_tx, status_rx) = tokio::sync::watch::channel(NetworkStatus::Starting);

    let (local_file_tx, local_file_rx) = watch::channel(vec![]);

    let network = Network {
        port,
        status: Arc::new(status_tx),
        local_files: local_file_rx,
    };

    runtime.spawn(async move {
        if let Err(report) = network.start().await {
            error!(?report);
        }
        network
            .status
            .send(NetworkStatus::Failed)
            .expect("failed to send failed status");
    });

    NetworkHandle {
        runtime,
        status: status_rx,
        local_files: local_file_tx,
    }
}

pub struct NetworkHandle {
    pub runtime: Runtime,
    pub status: watch::Receiver<NetworkStatus>,
    pub local_files: watch::Sender<Vec<LocalFile>>,
}

#[derive(PartialEq, Debug)]
pub enum NetworkStatus {
    Starting,
    Failed,
    Ok(Arc<Vec<RemoteFile>>),
}

struct Network {
    port: u16,
    status: Arc<watch::Sender<NetworkStatus>>,
    local_files: watch::Receiver<Vec<LocalFile>>,
}

impl Network {
    async fn start(&self) -> Result<()> {
        let discovery_recv_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, self.port);
        let discovery_recv_socket = UdpSocket::bind(discovery_recv_addr).await?;
        discovery_recv_socket.set_multicast_loop_v4(false)?;
        discovery_recv_socket.join_multicast_v4(IPV4_MULTICAST_ADDR, Ipv4Addr::UNSPECIFIED)?;

        let discovery_send_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0);
        let discovery_send_socket = UdpSocket::bind(discovery_send_addr).await?;
        discovery_send_socket.set_multicast_loop_v4(false)?;
        discovery_send_socket
            .connect(SocketAddrV4::new(IPV4_MULTICAST_ADDR, self.port))
            .await?;

        let send_handle = spawn_discovery_sender(&self.local_files, discovery_send_socket);

        let (remote_files_tx, mut remote_files_rx) = watch::channel(Arc::new(vec![]));

        let recv_handle = spawn_discovery_receiver(remote_files_tx, discovery_recv_socket);

        let transfer_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, self.port);
        let _transfer_socket = TcpListener::bind(transfer_addr).await?;

        let status = self.status.clone();
        let status_handle = tokio::spawn(async move {
            loop {
                remote_files_rx.changed().await.unwrap();
                let files = &*remote_files_rx.borrow_and_update();
                status.send(NetworkStatus::Ok(files.clone())).unwrap();
            }
        });

        tokio::try_join!(send_handle, recv_handle, status_handle)?;

        Ok(())
    }

    pub fn add_send(&self, path: &Path) {
        let _path = PathBuf::from(path);
        /*         self.send_tx
        .send(SendManagerMsg::Add(path))
        .wrap_err("failed to add send")
        .unwrap(); */
    }

    pub fn remove_send(&self, path: &Path) {
        let _path = PathBuf::from(path);
        //self.send_tx.send(SendManagerMsg::Remove(path)).unwrap();
    }
}
