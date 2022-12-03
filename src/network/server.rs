use std::{
    net::{Ipv4Addr, SocketAddrV4, SocketAddr},
    sync::Arc, path::Path,
};

use color_eyre::{eyre::WrapErr, Result};
use tokio::{sync::watch, io::{AsyncBufReadExt, AsyncWriteExt}};
use tracing::error;

use crate::common::LocalFile;

pub async fn run_file_download(addr: &SocketAddr, filename: &str, path: &Path) -> Result<()> {
    let mut stream = tokio::net::TcpStream::connect(addr).await.wrap_err("failed to connect")?;
    let mut filename = serde_json::to_vec(filename).wrap_err("failed to serialize filename")?;
    filename.push(b'\n');
    stream.write_all(&filename).await.wrap_err("failed to write filename to stream")?;
    let reader = tokio::io::BufReader::new(stream);
    let mut archive = tokio_tar::Archive::new(reader);
    archive.unpack(path).await.wrap_err("failed to unpack tar")?;
    Ok(())
}

pub async fn run_file_server(port: u16, local_files: watch::Receiver<Arc<Vec<LocalFile>>>) -> Result<()> {
    let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);
    let socket = tokio::net::TcpListener::bind(addr).await?;
    loop {
        let (stream, addr) = match socket.accept().await {
            Ok(result) => result,
            Err(err) => {
                error!("File server failed to accept: {}", err);
                continue;
            }
        };
        tracing::debug!("Client connected: {}", addr);
        let local_files = Arc::clone(&*local_files.borrow());
        tokio::spawn(async move {
            match run_connection(stream, local_files).await {
            Ok(_) => tracing::info!("Client completed: {}", addr),
            Err(err) => tracing::error!("Client failed: {} {}", addr, err),
        }});
    }
}

async fn run_connection(stream: tokio::net::TcpStream, local_files: Arc<Vec<LocalFile>>) -> Result<()> {
    let mut buf_stream = tokio::io::BufReader::new(stream);
    let mut filename = String::new();
    buf_stream
        .read_line(&mut filename)
        .await
        .wrap_err("failed to read filename from connection")?;
    let filename: String =
        serde_json::from_str(&filename).wrap_err("failed to parse filename as json")?;
    tracing::debug!("Received filename request: {}", filename);
    let file = local_files.iter().find(|f| f.name == filename);
    let file = match file {
        Some(file) => file,
        None => return Err(color_eyre::eyre::eyre!("filename not found: {}", filename)),
    };
    tracing::debug!("Found file at: {:?}", file.path.to_str());
    let stream = buf_stream.into_inner();
    let mut builder = tokio_tar::Builder::new(stream);
    if file.path.is_dir() {
        builder
            .append_dir_all(filename, file.path.as_path())
            .await
            .wrap_err("failed to write dir to tar builder")?;
    } else {
        let mut file = tokio::fs::File::open(file.path.as_path())
            .await
            .wrap_err("failed to open file that was to be written to tar builder")?;
        builder
            .append_file(filename, &mut file)
            .await
            .wrap_err("failed to write file to tar builder")?;
    }
    tracing::debug!("Finishing tar builder.");
    builder
        .finish()
        .await
        .wrap_err("failed to finish the tar builder")
}
