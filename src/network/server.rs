use std::{fmt::Write, sync::Arc, net::{SocketAddrV4, Ipv4Addr}};

use bytes::BytesMut;
use color_eyre::Result;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::watch,
};
use tracing::error;

use crate::common::LocalFile;

pub async fn run_file_server(
    port: u16,
    local_files: watch::Receiver<Arc<Vec<LocalFile>>>,
) -> Result<()> {
    let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);
    let socket = TcpListener::bind(addr).await?;
    let file_server = FileServer {
        socket,
        local_files,
    };
    loop {
        if let Err(report) = file_server.accept().await {
            error!("file server failed: {}", report);
        }
    }
}

struct FileServer {
    socket: TcpListener,
    local_files: watch::Receiver<Arc<Vec<LocalFile>>>,
}

impl FileServer {
    async fn accept(&self) -> Result<()> {
        loop {
            let (stream, _) = self.socket.accept().await?;
            let mut connection = Connection {
                stream,
                local_files: self.local_files.clone(),
            };
            tokio::spawn(async move {
                connection.handle().await;
            });
        }
    }
}

struct Connection {
    stream: TcpStream,
    local_files: watch::Receiver<Arc<Vec<LocalFile>>>,
}

impl Connection {
    async fn handle(&mut self) -> Result<()> {
        let mut buf = BytesMut::with_capacity(4096);
        loop {
            self.stream.read_buf(&mut buf).await?;
            if let Some(c) = buf.last() {
                if *c == b'\n' {
                    break;
                }
            }
        }
        let without_delimiter = (&buf[0..buf.len() - 1]).to_vec();
        let filename = String::from_utf8(without_delimiter)?;
        let local_files = (*self.local_files.borrow()).clone();
        buf.clear();
        match local_files.iter().find(|f| f.name == filename) {
            Some(file) => {
                buf.write_str("ok\n")?;
                self.stream.write_all_buf(&mut buf).await?;
            }
            None => {
                buf.write_str("not_found\n")?;
                self.stream.write_all_buf(&mut buf).await?;
            }
        }
        Ok(())
    }
}
