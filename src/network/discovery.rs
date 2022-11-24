use std::{
    cell::Cell,
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::common::{LocalFile, RemoteFile};
use bytes::{Buf, BytesMut};
use serde::{Deserialize, Serialize};
use tokio::{net::UdpSocket, sync::watch, task::JoinHandle, time::timeout};
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
                let packet = Packet { files };
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
    files_tx: watch::Sender<Arc<Vec<RemoteFile>>>,
    socket: UdpSocket,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut db: HashMap<SocketAddr, (Vec<String>, Cell<Instant>)> = HashMap::new();
        let mut buf = BytesMut::with_capacity(4096);

        fn map_remote_files(
            db: &HashMap<SocketAddr, (Vec<String>, Cell<Instant>)>,
        ) -> Arc<Vec<RemoteFile>> {
            Arc::new(
                db.iter()
                    .flat_map(|(addr, (files, _))| {
                        files.iter().map(|f| RemoteFile {
                            addr: addr.clone(),
                            file: f.clone(),
                        })
                    })
                    .collect(),
            )
        }

        loop {
            buf.clear();
            timeout(Duration::from_secs(1), socket.readable()).await.ok();

            // Handle timeouts
            let timeout_addrs: Vec<SocketAddr> = db
                .iter()
                .filter_map(|(addr, (_, time))| {
                    if time.get().elapsed() > Duration::from_secs(10) {
                        Some(addr.clone())
                    } else {
                        None
                    }
                })
                .collect();
            for addr in timeout_addrs.iter() {
                db.remove(addr);
            }
            if !timeout_addrs.is_empty() {
                files_tx.send(map_remote_files(&db)).unwrap();
            }

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
                    if let Some((files, time)) = db.get(&addr) {
                        if files == &packet.files {
                            time.replace(Instant::now());
                            continue;
                        }
                    }
                    db.insert(addr, (packet.files, Cell::new(Instant::now())));
                }
            }

            files_tx.send(map_remote_files(&db)).unwrap();
        }
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Packet {
    files: Vec<String>,
}
