mod test;

use bytes::{BufMut, BytesMut};
use color_eyre::eyre::{ensure, eyre, ContextCompat, WrapErr};
use color_eyre::{Report, Result};
use const_str::{concat, ip_addr};
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use lazy_static::lazy_static;
use network_interface::{NetworkInterface, NetworkInterfaceConfig};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::io::{Read, Write};
use std::net::SocketAddrV4;
use std::{
    ffi::{OsStr, OsString},
    net::Ipv4Addr,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::io::Interest;
use tokio::net::{TcpListener, UdpSocket};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::task::{spawn_blocking, JoinHandle};
use tracing::{error, info};

use crate::common::Key;

const IPV4_MULTICAST_ADDR: Ipv4Addr = ip_addr!(v4, "224.0.0.139");

pub fn run(key: Key, port: u16) -> NetworkHandle {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to create tokio runtime");

    let (status_tx, status_rx) = tokio::sync::watch::channel(NetworkStatus::Starting);

    let (local_file_tx, local_file_rx) = mpsc::channel(1024);

    let network = Network {
        key,
        port,
        status: status_tx,
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
    runtime: Runtime,
    status: watch::Receiver<NetworkStatus>,
    local_files: mpsc::Sender<()>,
}

#[derive(PartialEq, Clone, Debug)]
pub struct RemoteFile {
    key: Key,
    file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiscoveryPacket {
    key: Key,
    files: Vec<String>,
}

#[derive(PartialEq, Debug)]
pub enum NetworkStatus {
    Starting,
    Failed,
    Ok(Vec<RemoteFile>),
}

struct Network {
    key: Key,
    port: u16,
    status: watch::Sender<NetworkStatus>,
    local_files: mpsc::Receiver<()>,
}

impl Network {
    async fn start(&self) -> Result<()> {
        let discovery_recv_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, self.port);
        let discovery_recv_socket = UdpSocket::bind(discovery_recv_addr).await?;
        discovery_recv_socket.join_multicast_v4(IPV4_MULTICAST_ADDR, Ipv4Addr::UNSPECIFIED)?;

        let discovery_send_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0);
        let discovery_send_socket = UdpSocket::bind(discovery_send_addr).await?;
        discovery_send_socket.set_multicast_loop_v4(false)?;
        discovery_send_socket
            .connect(SocketAddrV4::new(IPV4_MULTICAST_ADDR, self.port))
            .await?;

        let key = self.key.clone();

        let send_handle = tokio::spawn(async move {
            loop {
                let files = vec![];
                let packet = DiscoveryPacket {
                    key: key.clone(),
                    files,
                };
                let buf = serde_json::to_string(&packet).unwrap();
                discovery_send_socket.send(buf.as_bytes()).await.unwrap();
                info!("Sent: {buf}");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });

        let recv_handle = tokio::spawn(async move {
            loop {
                let mut buf = BytesMut::with_capacity(4096);
                discovery_recv_socket.readable().await.unwrap();
                let result = discovery_recv_socket.try_recv_buf_from(&mut buf);
                if let Err(err) = &result {
                    if err.kind() == std::io::ErrorKind::WouldBlock {
                        continue;
                    }
                }
                let (_, addr) = result.unwrap();
                let data = String::from_utf8(buf.to_vec()).unwrap();
                info!("Received from {addr}: {data}");
            }
        });

        let transfer_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, self.port);
        let transfer_socket = TcpListener::bind(transfer_addr).await?;

        tokio::try_join!(send_handle, recv_handle)?;

        Ok(())
    }

    pub fn add_send(&self, path: &Path) {
        let path = PathBuf::from(path);
        /*         self.send_tx
        .send(SendManagerMsg::Add(path))
        .wrap_err("failed to add send")
        .unwrap(); */
    }

    pub fn remove_send(&self, path: &Path) {
        let path = PathBuf::from(path);
        //self.send_tx.send(SendManagerMsg::Remove(path)).unwrap();
    }
}

enum SendManagerMsg {
    Add(PathBuf),
    Remove(PathBuf),
}

struct Send {
    path: PathBuf,
    name: String,
}

#[derive(Serialize, Deserialize)]
struct SendPacket {
    names: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct SendRequestPacket {
    name: String,
}

struct SendManager {
    multicast_socket_v4: UdpSocket,
    multicast_socket_addr_v4: SocketAddrV4,
    sends: Vec<Send>,
    rx: Receiver<SendManagerMsg>,
    listener: TcpListener,
    buf: Vec<u8>,
}

/* impl SendManager {
    fn run(
        multicast_socket_v4: UdpSocket,
        multicast_socket_addr_v4: SocketAddrV4,
        rx: Receiver<SendManagerMsg>,
        listener: TcpListener,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            let mut manager = SendManager {
                multicast_socket_v4,
                multicast_socket_addr_v4,
                sends: vec![],
                rx,
                listener,
                buf: vec![],
            };
            loop {
                manager.update();
            }
        })
    }

    fn update(&mut self) {
        match self.rx.try_recv() {
            Err(TryRecvError::Disconnected) => panic!("disconnected"),
            Err(TryRecvError::Empty) => (),
            Ok(SendManagerMsg::Add(path)) => {
                if let Err(report) = self.add(path) {
                    info!("{report}");
                }
            }
            Ok(SendManagerMsg::Remove(path)) => self.remove(path),
        }
        self.send_multicast();
        self.accept();
        thread::sleep(Duration::from_millis(1000));
    }

    fn add(&mut self, path: PathBuf) -> Result<()> {
        let name = path
            .file_name()
            .wrap_err("no filename in path")?
            .to_str()
            .wrap_err("filename not unicode")?
            .to_string();

        for send in self.sends.iter() {
            ensure!(send.path != path, "send with path already exists");
            ensure!(send.name != name, "send with name already exists");
        }
        self.sends.push(Send { path, name });
        Ok(())
    }

    fn remove(&mut self, path: PathBuf) {
        for i in 0..self.sends.len() {
            if self.sends[i].path == path {
                self.sends.remove(i);
                break;
            }
        }
    }

    fn send_multicast(&mut self) {
        if self.sends.is_empty() {
            return;
        }
        let names = self
            .sends
            .iter()
            .filter_map(|s| s.path.file_name())
            .filter_map(|s| s.to_str())
            .map(|s| String::from(s))
            .collect();
        let packet = SendPacket { names };
        //serde_json::to_writer(writer, value)
        let serialized = serde_json::to_string(&packet).unwrap();
        let buf = serialized.as_bytes();
        self.multicast_socket_v4
            .send_to(buf, self.multicast_socket_addr_v4)
            .unwrap();
    }

    fn accept(&mut self) {
        loop {
            match self.listener.accept() {
                Ok((stream, addr)) => {
                    info!("Accepting connection from: {}", addr);
                    self.start(stream).unwrap();
                    /*
                    if let Err(report) = self.start(stream) {
                        info!("Failed to handle request: {}", report)
                    }
                    */
                }
                Err(error) => {
                    if let std::io::ErrorKind::WouldBlock = error.kind() {
                        break;
                    } else {
                        panic!("{}", error);
                    }
                }
            }
        }
    }

    fn start(&mut self, mut stream: TcpStream) -> Result<()> {
        stream
            .set_nonblocking(false)
            .wrap_err("failed to set nonblocking")?;
        //stream.set_read_timeout(Some(Duration::from_millis(10000))).wrap_err("failed to set read timeout")?;
        let request: SendRequestPacket =
            serde_json::from_reader(&stream).wrap_err("failed to read stream")?;
        let path = self
            .sends
            .iter()
            .find(|s| s.name == request.name)
            .wrap_err("requested name does not exist")?
            .path
            .as_path();
        info!("Starting to send {:?} to stream.", request.name);
        let buf = "test".as_bytes();
        stream.write(buf)?;
        Ok(())
    }
} */

struct SharyProtocolWriter {
    buf: Vec<u8>,
}

impl Write for SharyProtocolWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut size: usize = 0;
        size += self.buf.write(&buf.len().to_be_bytes())?;
        size += self.buf.write(b"\n")?;
        size += self.buf.write(buf)?;
        Ok(size)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.buf.flush()
    }
}
