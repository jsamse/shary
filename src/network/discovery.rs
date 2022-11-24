use std::{net::SocketAddr, time::Duration};

use crate::common::LocalFile;
use bytes::{Buf, BytesMut};
use serde::{Deserialize, Serialize};
use tokio::{
    net::UdpSocket,
    sync::{mpsc, watch},
    task::JoinHandle,
};
use tracing::debug;

#[derive(Debug)]
pub struct RemoteFiles {
    pub files: Vec<String>,
    pub addr: SocketAddr,
}

pub fn spawn_discovery_sender(
    files_rx: &watch::Receiver<Vec<LocalFile>>,
    socket: UdpSocket,
) -> JoinHandle<()> {
    let mut files_rx = files_rx.clone();
    tokio::spawn(async move {
        let mut buf: Vec<u8> = vec![];
        loop {
            if buf.is_empty() || files_rx.has_changed().unwrap() {
                let local_files = &*files_rx.borrow_and_update();
                let files: Vec<String> = local_files.iter().map(|l| l.name.clone()).collect();
                let packet = Packet {
                    files,
                };
                buf = serde_json::to_vec(&packet).unwrap();
            }
            socket.send(&buf).await.unwrap();
            let json = String::from_utf8(buf.to_vec()).unwrap();
            debug!("Sent discovery packet: {json}");
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    })
}

pub fn spawn_discovery_receiver(
    files_tx: &mpsc::Sender<RemoteFiles>,
    socket: UdpSocket,
) -> JoinHandle<()> {
    let files_tx = files_tx.clone();
    tokio::spawn(async move {
        let mut buf = BytesMut::with_capacity(4096);
        loop {
            buf.clear();
            socket.readable().await.unwrap();
            let result = socket.try_recv_buf_from(&mut buf);
            if let Err(err) = &result {
                if err.kind() == std::io::ErrorKind::WouldBlock {
                    continue;
                }
            }
            let (_, addr) = result.unwrap();
            let mut reader = buf.reader();
            let result: Option<Packet> = serde_json::from_reader(&mut reader).ok();
            buf = reader.into_inner();
            match result {
                None => continue,
                Some(packet) => {
                    debug!("Received from {addr}: {:?}", packet);
                    let remote_files = RemoteFiles {
                        files: packet.files,
                        addr,
                    };
                    files_tx.send(remote_files).await.unwrap();
                }
            }
        }
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Packet {
    files: Vec<String>,
}
