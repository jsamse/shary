#[cfg(test)]
use crate::{
    common::{LocalFile, RemoteFile},
    network::discovery::{spawn_discovery_receiver, spawn_discovery_sender},
};
use const_str::ip_addr;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::{net::{Ipv6Addr, SocketAddr, IpAddr}, path::PathBuf, sync::Arc};
use tokio::{
    net::UdpSocket,
    sync::{mpsc, watch},
};

#[tokio::test]
async fn broadcast_is_received() {
    let sender = UdpSocket::bind("0.0.0.0:17600").await.unwrap();
    sender.set_broadcast(true).unwrap();
    let receiver = UdpSocket::bind("0.0.0.0:17700").await.unwrap();
    receiver.set_broadcast(true).unwrap();
    sender
        .send_to("test".as_bytes(), "255.255.255.255:17700")
        .await
        .unwrap();
    let mut buf = vec![0; 10];
    let (size, addr) = receiver.recv_from(&mut buf).await.unwrap();
    let word = Vec::from(&buf[0..size]);
    println!(
        "{:?} {:?} {:?}",
        size,
        String::from_utf8(word).unwrap(),
        addr
    );
}

#[tokio::test]
async fn ipv4_multicast() {
    let multicast_addr = ip_addr!(v4, "224.0.0.139");
    let first = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 17771))
        .await
        .unwrap();
    first
        .join_multicast_v4(multicast_addr, Ipv4Addr::UNSPECIFIED)
        .unwrap();

    let second = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 17772))
        .await
        .unwrap();
    second
        .join_multicast_v4(multicast_addr, Ipv4Addr::UNSPECIFIED)
        .unwrap();

    let mut buf = "test".as_bytes().to_vec();
    first.send_to(&buf, (multicast_addr, 17772)).await.unwrap();
    buf.fill(0);
    let (size, addr) = second.recv_from(&mut buf).await.unwrap();
    println!(
        "Received: {:?} {:?} {:?}",
        size,
        String::from_utf8((&buf[0..size]).to_vec()).unwrap(),
        addr
    );
}

#[tokio::test]
async fn ipv6_multicast() {
    return; // Need to bind to a specific interface on MacOS. Will not use ipv6 for now.
    let multicast_addr = ip_addr!(v6, "ff02::139");
    assert!(multicast_addr.is_multicast());
    let first = UdpSocket::bind((Ipv6Addr::UNSPECIFIED, 17773))
        .await
        .unwrap();
    first.join_multicast_v6(&multicast_addr, 0).unwrap();

    let second = UdpSocket::bind((Ipv6Addr::UNSPECIFIED, 17774))
        .await
        .unwrap();
    second.join_multicast_v6(&multicast_addr, 0).unwrap();

    let mut buf = "test".as_bytes().to_vec();
    first.send_to(&buf, (multicast_addr, 17774)).await.unwrap();
    buf.fill(0);
    let (size, addr) = second.recv_from(&mut buf).await.unwrap();
    println!(
        "Received: {:?} {:?} {:?}",
        size,
        String::from_utf8((&buf[0..size]).to_vec()).unwrap(),
        addr
    );
}

#[tokio::test]
async fn json_newline() {
    let item = std::collections::HashMap::from([("filename", "test1\ntest2")]);

    let json = serde_json::to_string(&item).unwrap();

    assert_eq!(r#"{"filename":"test1\ntest2"}"#, json);
}

#[tokio::test]
async fn discovery() {
    let recv_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 17891);
    let recv_socket = UdpSocket::bind(&recv_addr).await.unwrap();

    let send_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 17892);
    let send_socket = UdpSocket::bind(&send_addr).await.unwrap();
    send_socket.connect(("127.0.0.1", 17891)).await.unwrap();

    let (local_files_tx, local_files_rx) = watch::channel(Arc::new(vec![
        LocalFile {
            path: PathBuf::new(),
            name: String::from("test1"),
        },
        LocalFile {
            path: PathBuf::new(),
            name: String::from("test2"),
        },
    ]));

    spawn_discovery_sender(&local_files_rx, send_socket);

    let (remote_files_tx, mut remote_files_rx) = watch::channel(Arc::new(vec![]));

    spawn_discovery_receiver(remote_files_tx, recv_socket);

    remote_files_rx.changed().await.unwrap();
    let remote_files = (&*remote_files_rx.borrow_and_update()).clone();

    assert_eq!(
        Arc::new(vec![
            RemoteFile {
                addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 17892),
                file: String::from("test1"),
            },
            RemoteFile {
                addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 17892),
                file: String::from("test2"),
            },
        ]),
        remote_files
    );

    local_files_tx.send(Arc::new(vec![])).unwrap();

    remote_files_rx.changed().await.unwrap();
    let remote_files = (&*remote_files_rx.borrow_and_update()).clone();

    assert!(remote_files.is_empty());
}
