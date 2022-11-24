use std::{path::PathBuf, net::SocketAddr};

use color_eyre::{eyre::eyre, eyre::Context, Report};
use tokio::{sync::watch, task::JoinHandle};

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

pub async fn spawn_mapper<R: Clone + Send + Sync + 'static, S: Clone + Send + Sync + 'static, F: Fn(R) -> S + Send + Sync + 'static>(
    recv: &watch::Receiver<R>,
    send: watch::Sender<S>,
    map: F,
) -> JoinHandle<Report> {
    let mut recv = recv.clone();
    tokio::spawn(async move {
        loop {
            if let Err(report) = recv.changed().await.wrap_err("failed to receive") {
                return report;
            }
            let original = (&*recv.borrow_and_update()).clone();
            let mapped = map(original);
            if let Err(_) = send.send(mapped) {
                return eyre!("failed to send");
            }
        }
    })
}
