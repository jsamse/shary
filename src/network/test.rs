use std::collections::HashMap;
#[cfg(test)]
use std::net::Ipv6Addr;

use super::*;

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
    let item = HashMap::from([("filename", "test1\ntest2")]);

    let json = serde_json::to_string(&item).unwrap();

    assert_eq!(r#"{"filename":"test1\ntest2"}"#, json);
}

#[tokio::test]
async fn discovery_sender() {
    let recv_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 17891);
    let recv_socket = UdpSocket::bind(recv_addr).await.unwrap();

    let send_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 17892);
    let send_socket = UdpSocket::bind(send_addr).await.unwrap();
    send_socket.connect(("127.0.0.1", 17891)).await.unwrap();

    let key = Key::new();

    let (files_tx, files_rx) = watch::channel(vec![]);

    spawn_discovery_sender(&key, files_rx, send_socket);

    let mut buf = vec![0; 4096];

    let (size, _) = recv_socket.recv_from(&mut buf).await.unwrap();
    let result: DiscoveryPacket = serde_json::from_slice(&buf[..size]).unwrap();
    assert!(result.files.is_empty());

    files_tx.send(vec![String::from("test")]).unwrap();

    let (size, _) = recv_socket.recv_from(&mut buf).await.unwrap();
    let result: DiscoveryPacket = serde_json::from_slice(&buf[..size]).unwrap();
    assert_eq!(vec![String::from("test")], result.files);
}
