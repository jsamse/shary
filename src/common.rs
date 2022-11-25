use std::{path::PathBuf, net::SocketAddr};

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
