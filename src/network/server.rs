use std::{
    net::{Ipv4Addr, SocketAddrV4},
    path::PathBuf,
};

use color_eyre::{eyre::WrapErr, Result};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufWriter},
    sync::watch,
};
use tracing::error;

use crate::common::{Files, LocalFile, RemoteFile, DownloadStatus};

pub async fn run_file_download(files: &Files) -> Result<()> {
    let mut downloads = files.get_downloads();
    loop {
        let (remote_file, path) = downloads
            .recv()
            .await
            .wrap_err("download channel sender closed")?;
        files.set_download_status(remote_file.clone(), Some(DownloadStatus::Running));
        let result = download(remote_file.clone(), path).await;
        let status = match result {
            Ok(_) => DownloadStatus::Completed,
            Err(report) => DownloadStatus::Failed(report.to_string()),
        };
        files.set_download_status(remote_file, Some(status));
    }
}

async fn download(remote_file: RemoteFile, path: PathBuf) -> Result<()> {
    let mut stream = tokio::net::TcpStream::connect(remote_file.addr)
        .await
        .wrap_err("failed to connect")?;
    let mut filename =
        serde_json::to_vec(&remote_file.file).wrap_err("failed to serialize filename")?;
    filename.push(b'\n');
    stream
        .write_all(&filename)
        .await
        .wrap_err("failed to write filename to stream")?;
    let reader = tokio::io::BufReader::new(stream);
    let mut archive = tokio_tar::Archive::new(reader);
    archive
        .unpack(path)
        .await
        .wrap_err("failed to unpack tar")
}

pub async fn run_file_server(
    port: u16,
    local_files: watch::Receiver<Vec<LocalFile>>,
) -> Result<()> {
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
        let local_files = local_files.borrow().clone();
        tokio::spawn(async move {
            match run_connection(stream, local_files).await {
                Ok(_) => tracing::info!("Client completed: {}", addr),
                Err(err) => tracing::error!("Client failed: {} {}", addr, err),
            }
        });
    }
}

async fn run_connection(stream: tokio::net::TcpStream, local_files: Vec<LocalFile>) -> Result<()> {
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
    let buf_writer = BufWriter::new(stream);
    let mut builder = tokio_tar::Builder::new(buf_writer);
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
