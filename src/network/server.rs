use std::{
    io::{BufRead, BufReader},
    net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream},
    sync::Arc,
};

use color_eyre::{eyre::WrapErr, Result};
use tokio::sync::watch;
use tracing::error;

use crate::common::LocalFile;

pub fn run_file_server(port: u16, local_files: watch::Receiver<Arc<Vec<LocalFile>>>) -> Result<()> {
    let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);
    let socket = TcpListener::bind(addr)?;
    loop {
        let (stream, addr) = match socket.accept() {
            Ok(result) => result,
            Err(err) => {
                error!("File server failed to accept: {}", err);
                continue;
            }
        };
        tracing::info!("Client connected: {}", addr);
        let local_files = Arc::clone(&*local_files.borrow());
        std::thread::spawn(move || match run_connection(stream, local_files) {
            Ok(_) => tracing::info!("Client completed: {}", addr),
            Err(err) => tracing::error!("Client failed: {} {}", addr, err),
        });
    }
}

fn run_connection(stream: TcpStream, local_files: Arc<Vec<LocalFile>>) -> Result<()> {
    let mut buf_stream = BufReader::new(stream);
    let mut filename = String::new();
    buf_stream
        .read_line(&mut filename)
        .wrap_err("failed to read filename from connection")?;
    let filename: String =
        serde_json::from_str(&filename).wrap_err("failed to parse filename as json")?;
    let file = local_files.iter().find(|f| f.name == filename);
    let file = match file {
        Some(file) => file,
        None => return Err(color_eyre::eyre::eyre!("filename not found: {}", filename)),
    };
    let stream = buf_stream.into_inner();
    let mut builder = tar::Builder::new(stream);
    if file.path.is_dir() {
        builder
            .append_dir_all(filename, file.path.as_path())
            .wrap_err("failed to write dir to tar builder")?;
    } else {
        let mut file = std::fs::File::open(file.path.as_path())
            .wrap_err("failed to open file that was to be written to tar builder")?;
        builder
            .append_file(filename, &mut file)
            .wrap_err("failed to write file to tar builder")?;
    }
    builder
        .finish()
        .wrap_err("failed to finish the tar builder")
}
