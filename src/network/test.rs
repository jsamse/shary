use crate::network::IPV4_MULTICAST_ADDR;
use crate::{
    common::{LocalFile},
    network::discovery::{run_discovery_receiver, run_discovery_sender},
};
use std::{net::{Ipv4Addr, SocketAddrV4}, time::Duration};
use std::{path::PathBuf, sync::Arc};
use tokio::{
    sync::watch,
};

#[tokio::test]
async fn discovery() {
    let (local_files_tx, local_files_rx) = watch::channel(vec![
        LocalFile {
            path: PathBuf::new(),
            name: String::from("test1"),
        },
        LocalFile {
            path: PathBuf::new(),
            name: String::from("test2"),
        },
    ]);

    tokio::spawn(async move {
        run_discovery_sender(local_files_rx, SocketAddrV4::new(Ipv4Addr::LOCALHOST, 17891)).await.unwrap();
    });

    let (remote_files_tx, mut remote_files_rx) = watch::channel(Arc::new(vec![]));

    tokio::spawn(async move {
        run_discovery_receiver(&remote_files_tx, 17891, IPV4_MULTICAST_ADDR).await.unwrap();
    });

    remote_files_rx.changed().await.unwrap();

    let remote_files = (&*remote_files_rx.borrow_and_update()).clone();

    assert_eq!(2, remote_files.len());
    assert_eq!(Ipv4Addr::LOCALHOST, remote_files[0].addr.ip());
    assert_eq!(String::from("test1"), remote_files[0].file);
    assert_eq!(Ipv4Addr::LOCALHOST, remote_files[1].addr.ip());
    assert_eq!(String::from("test2"), remote_files[1].file);

    local_files_tx.send(vec![]).unwrap();

    remote_files_rx.changed().await.unwrap();

    let remote_files = (&*remote_files_rx.borrow_and_update()).clone();

    assert!(remote_files.is_empty());
}

#[tokio::test]
async fn discovery_timeout() {
    let (local_files_tx, local_files_rx) = watch::channel(vec![
        LocalFile {
            path: PathBuf::new(),
            name: String::from("test1"),
        },
    ]);

    tokio::spawn(async move {
        run_discovery_sender(local_files_rx, SocketAddrV4::new(Ipv4Addr::LOCALHOST, 17891)).await.unwrap();
    });

    let (remote_files_tx, mut remote_files_rx) = watch::channel(Arc::new(vec![]));

    tokio::spawn(async move {
        run_discovery_receiver(&remote_files_tx, 17891, IPV4_MULTICAST_ADDR).await.unwrap();
    });

    remote_files_rx.changed().await.unwrap();

    let remote_files = (&*remote_files_rx.borrow_and_update()).clone();

    assert_eq!(1, remote_files.len());
    assert_eq!(Ipv4Addr::LOCALHOST, remote_files[0].addr.ip());
    assert_eq!(String::from("test1"), remote_files[0].file);

    std::mem::drop(local_files_tx);

    tokio::time::sleep(Duration::from_secs(7)).await;

    assert!(remote_files_rx.has_changed().unwrap());

    let remote_files = (&*remote_files_rx.borrow_and_update()).clone();

    assert!(remote_files.is_empty());
}