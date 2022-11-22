mod test;

use color_eyre::eyre::{ensure, eyre, ContextCompat, WrapErr};
use color_eyre::Result;
use const_str::{concat, ip_addr};
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use lazy_static::lazy_static;
use network_interface::{NetworkInterface, NetworkInterfaceConfig};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::net::SocketAddrV4;
use std::{
    ffi::{OsStr, OsString},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream, UdpSocket},
    path::{Path, PathBuf},
    thread::{self, JoinHandle},
    time::Duration,
};
use tracing::{error, info};

const IPV4_MULTICAST_ADDR: Ipv4Addr = ip_addr!(v4, "224.0.0.139");

pub struct Network {
    send_tx: Sender<SendManagerMsg>,
    send_handle: JoinHandle<()>,
}

impl Network {
    pub fn new(port: u16) -> Result<Network> {
        let bind_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);
        let listener = TcpListener::bind(bind_addr).wrap_err("failed to bind tcp listener")?;
        listener
            .set_nonblocking(true)
            .wrap_err("failed to set nonblocking")?;
        let multicast_socket_v4 = UdpSocket::bind(bind_addr).wrap_err("failed to bind broadcast socket")?;
        multicast_socket_v4
            .join_multicast_v4(&IPV4_MULTICAST_ADDR, &Ipv4Addr::UNSPECIFIED)
            .wrap_err("failed to set broadcast")?;
        multicast_socket_v4
            .set_read_timeout(Some(Duration::from_millis(1000)))
            .wrap_err("failed to set read timeout")?;
        let multicast_socket_addr_v4 = SocketAddrV4::new(IPV4_MULTICAST_ADDR, port);
        let (send_tx, send_rx) = crossbeam_channel::unbounded();
        let send_handle = SendManager::run(multicast_socket_v4, multicast_socket_addr_v4, send_rx, listener);
        Ok(Network {
            send_tx,
            send_handle,
        })
    }

    pub fn add_send(&self, path: &Path) {
        let path = PathBuf::from(path);
        self.send_tx
            .send(SendManagerMsg::Add(path))
            .wrap_err("failed to add send")
            .unwrap();
    }

    pub fn remove_send(&self, path: &Path) {
        let path = PathBuf::from(path);
        self.send_tx.send(SendManagerMsg::Remove(path)).unwrap();
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

impl SendManager {
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
        self.multicast_socket_v4.send_to(buf, self.multicast_socket_addr_v4).unwrap();
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
}

struct SharyProtocolWriter {
    buf: Vec<u8>
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