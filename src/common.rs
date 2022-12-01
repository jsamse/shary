use std::{net::SocketAddr, path::PathBuf};

#[derive(PartialEq, Clone, Debug)]
pub struct LocalFile {
    pub path: PathBuf,
    pub name: String,
}

#[derive(PartialEq, Clone, Debug)]
pub struct RemoteFile {
    pub addr: SocketAddr,
    pub file: String,
}
