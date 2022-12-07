use std::{net::SocketAddr, path::PathBuf, sync::Arc, collections::HashMap};

use color_eyre::{Result, eyre::eyre};
use tokio::sync::{broadcast, watch};

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct LocalFile {
    pub path: PathBuf,
    pub name: String,
}

impl LocalFile {
    pub fn new(path: PathBuf) -> Result<LocalFile> {
        let file_name = path.file_name().ok_or(eyre!("no filename found for path"))?;
        let os_str = file_name.to_str().ok_or(eyre!("filename not valid utf8"))?;
        let name = os_str.to_owned();
        Ok(LocalFile { path, name })
    }
}

#[derive(Eq, PartialEq, Clone, Debug, Hash)]
pub struct RemoteFile {
    pub addr: SocketAddr,
    pub file: String,
}

#[derive(Eq, PartialEq, Clone, Debug, Hash)]
pub enum DownloadStatus {
    Running,
    Completed,
    Failed(String),
}

pub struct Files {
    local_files_tx: watch::Sender<Vec<LocalFile>>,
    pub remote_files_tx: watch::Sender<Arc<Vec<RemoteFile>>>,
    downloads_tx: broadcast::Sender<(RemoteFile, PathBuf)>,
    download_status_tx: watch::Sender<HashMap<RemoteFile, DownloadStatus>>,
}

impl Default for Files {
    fn default() -> Self {
        let (local_files_tx, _) = watch::channel(vec![]);
        let (remote_files_tx, _) = watch::channel(Arc::new(vec![]));
        let (downloads_tx, _) = broadcast::channel(1);
        let (download_status_tx, _) = watch::channel(HashMap::new());
        Self {
            local_files_tx,
            remote_files_tx,
            downloads_tx,
            download_status_tx,
        }
    }
}

impl Files {
    pub fn add_local_file(&self, local_file: LocalFile) -> bool {
        self.local_files_tx.send_if_modified(|local_files| {
            if local_files.contains(&local_file) {
                false
            } else {
                local_files.push(local_file);
                true
            }
        })
    }

    pub fn remove_local_file(&self, local_file: &LocalFile) -> bool {
        self.local_files_tx.send_if_modified(|local_files| {
            if let Some((i, _)) = local_files
                .iter()
                .enumerate()
                .find(|(_, f)| *f == local_file)
            {
                local_files.remove(i);
                true
            } else {
                false
            }
        })
    }

    pub fn get_local_files(&self) -> watch::Receiver<Vec<LocalFile>> {
        self.local_files_tx.subscribe()
    }

    pub fn get_remote_files(&self) -> watch::Receiver<Arc<Vec<RemoteFile>>> {
        self.remote_files_tx.subscribe()
    }

    pub fn add_download(&self, remote_file: RemoteFile, path: PathBuf) {
        let _ = self.downloads_tx.send((remote_file, path));
    }

    pub fn get_downloads(&self) -> broadcast::Receiver<(RemoteFile, PathBuf)> {
        self.downloads_tx.subscribe()
    }

    pub fn set_download_status(&self, remote_file: RemoteFile, status: Option<DownloadStatus>) {
        match status {
            Some(status) => self.download_status_tx.send_if_modified(|m| {
                m.insert(remote_file, status);
                true
            }),
            None => self.download_status_tx.send_if_modified(|m| {
                m.remove(&remote_file);
                true
            }),
        };
    }

    pub fn get_download_status(&self, remote_file: &RemoteFile) -> Option<DownloadStatus> {
        self.download_status_tx.borrow().get(remote_file).cloned()
    }
}

#[macro_export]
macro_rules! some_or_continue {
    ($e:expr) => {
        match $e {
            Some(v) => v,
            None => continue,
        }
    };
}

#[macro_export]
macro_rules! ok_or_continue {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(_) => continue,
        }
    };
}
