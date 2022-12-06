//! This module contains functions to send and receive device discovery data.

use std::{
    cell::Cell,
    collections::HashMap,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::Arc,
    time::{Duration, Instant},
};

use crate::common::{LocalFile, RemoteFile};
use bytes::{Buf, BufMut, BytesMut};
use color_eyre::{eyre::WrapErr, Result};
use serde::{Deserialize, Serialize};
use tokio::{net::UdpSocket, sync::watch, time::timeout};

/// Contains the information about the files that someone else is sharing.
#[derive(Debug)]
pub struct RemoteFiles {
    pub files: Vec<String>,
    pub addr: SocketAddr,
}

/// Periodically sends the current local files to the supplied socket address.
/// If nothing fails, the function will never return.
/// If the connected sender is dropped, this function will return [Ok(())].
pub async fn run_discovery_sender(
    mut files_rx: watch::Receiver<Vec<LocalFile>>,
    addr: SocketAddrV4,
) -> Result<()> {
    let bind_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0);
    let socket = UdpSocket::bind(bind_addr)
        .await
        .wrap_err("failed to bind socket")?;
    socket
        .set_multicast_loop_v4(false)
        .wrap_err("failed to set multicast looping to off")?;
    socket
        .connect(addr)
        .await
        .wrap_err("failed to connect socket")?;
    let mut buf = BytesMut::new();
    loop {
        let update_buffer = if buf.is_empty() {
            true
        } else {
            match files_rx.has_changed() {
                Ok(changed) => changed,
                Err(_) => return Ok(()), // files_rx is closed and we can stop
            }
        };
        if update_buffer {
            tracing::debug!("Writing discovery files to send buffer.");
            buf.clear();
            let files: Vec<String> = files_rx.borrow_and_update().iter().map(|l| l.name.clone()).collect();
            let packet = Packet { files };
            let mut writer = buf.writer();
            let json_result = serde_json::to_writer(&mut writer, &packet);
            buf = writer.into_inner();
            if let Err(error) = json_result {
                buf.clear();
                tracing::error!("Failed to format packet to json: {}", error);
            }
        }
        if !buf.is_empty() {
            socket
                .send(&buf)
                .await
                .wrap_err("failed to write discovery files to socket")?;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

/// Receives remote files using a multicast address.
/// If nothing fails, the function will never return.
/// If the connected receiver is dropped, this function will return [Ok(())].
pub async fn run_discovery_receiver(
    files_tx: &watch::Sender<Arc<Vec<RemoteFile>>>,
    port: u16,
    multicast_addr: Ipv4Addr,
) -> Result<()> {
    let bind_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);
    let socket = UdpSocket::bind(bind_addr)
        .await
        .wrap_err("failed to bind socket to addr")?;
    socket
        .set_multicast_loop_v4(false)
        .wrap_err("failed to set multicast looping to false")?;
    socket
        .join_multicast_v4(multicast_addr, Ipv4Addr::UNSPECIFIED)
        .wrap_err("failed to join multicast")?;

    let mut db: HashMap<SocketAddr, (Vec<String>, Cell<Instant>)> = HashMap::new();
    let mut buf = BytesMut::with_capacity(64000);

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
        timeout(Duration::from_secs(1), socket.readable())
            .await
            .ok();

        // Handle timeouts
        let timeout_addrs: Vec<SocketAddr> = db
            .iter()
            .filter_map(|(addr, (_, time))| {
                if time.get().elapsed() > Duration::from_secs(5) {
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
            if let Err(_) = files_tx.send(map_remote_files(&db)) {
                return Ok(());
            }
        }

        let mut addr = match socket.try_recv_buf_from(&mut buf) {
            Ok((_, addr)) => addr,
            Err(err) => {
                if err.kind() != tokio::io::ErrorKind::WouldBlock {
                    tracing::error!("Failed to read from discovery socket: {}", err);
                }
                continue;
            }
        };
        addr.set_port(port);
        let mut reader = buf.reader();
        let result: Result<Packet, serde_json::Error> = serde_json::from_reader(&mut reader);
        buf = reader.into_inner();
        match result {
            Err(err) => {
                tracing::error!("Failed to parse discovery json: {}", err);
                continue;
            }
            Ok(packet) => {
                tracing::debug!("Received from {addr}: {:?}", packet);
                if let Some((files, time)) = db.get(&addr) {
                    if files == &packet.files {
                        time.replace(Instant::now());
                        continue;
                    }
                }
                db.insert(addr, (packet.files, Cell::new(Instant::now())));
            }
        }

        if let Err(_) = files_tx.send(map_remote_files(&db)) {
            return Ok(());
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Packet {
    files: Vec<String>,
}
