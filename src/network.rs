use color_eyre::eyre::WrapErr;
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use network_interface::{NetworkInterface, NetworkInterfaceConfig};
use std::{
    net::{UdpSocket, IpAddr, ToSocketAddrs},
    path::{Path, PathBuf},
    thread::{self, JoinHandle},
    time::Duration,
};
use tracing::info;

use color_eyre::Result;

const bind_addr: &str = "0.0.0.0:17500";

pub struct Network {
    send_tx: Sender<SendManagerMsg>,
    send_handle: JoinHandle<()>,
}

impl Network {
    pub fn new() -> Result<Network> {
        let socket = UdpSocket::bind(bind_addr).wrap_err("failed to bind broadcast socket")?;
        socket.set_broadcast(true)?;
        let (send_tx, send_rx) = crossbeam_channel::unbounded();
        let send_handle = SendManager::run(socket, send_rx);
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
}

struct SendManager {
    socket: UdpSocket,
    sends: Vec<Send>,
    rx: Receiver<SendManagerMsg>,
}

impl SendManager {
    fn run(socket: UdpSocket, rx: Receiver<SendManagerMsg>) -> JoinHandle<()> {
        thread::spawn(move || {
            let mut manager = SendManager {
                socket,
                sends: vec![],
                rx,
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
            Ok(SendManagerMsg::Add(path)) => self.add(path),
            Ok(SendManagerMsg::Remove(path)) => self.remove(path),
        }
        self.broadcast();
        thread::sleep(Duration::from_millis(1000));
    }

    fn add(&mut self, path: PathBuf) {
        if self.sends.iter().any(|s| s.path == path) {
            return;
        }
        self.sends.push(Send { path })
    }

    fn remove(&mut self, path: PathBuf) {
        for i in 0..self.sends.len() {
            if self.sends[i].path == path {
                self.sends.remove(i);
                break;
            }
        }
    }

    fn broadcast(&mut self) {
        let interfaces = network_interface::NetworkInterface::show().unwrap();
        let broadcasts = interfaces.iter().filter_map(get_broadcast);
        for addr in broadcasts {
            let buf = "test".as_bytes();
            info!("Sending to: {:?}", addr);
            let addr: (IpAddr, u16) = (addr, 0);
            //self.socket.send(buf).unwrap();
            self.socket.send_to(buf, "255.255.255.255:17500").unwrap();
        }
        let mut buf = vec![];
        self.socket.set_read_timeout(Some(Duration::from_millis(1000))).unwrap();
        self.socket.recv(&mut buf).unwrap();
        panic!("{}", String::from_utf8(buf).unwrap());
    }
}

fn get_broadcast(i: &NetworkInterface) -> Option<IpAddr> {
    let addr = i.addr?;
    let broadcast = addr.broadcast()?;
    if !broadcast.is_ipv4() {
        return None;
    }
    Some(broadcast)
}
