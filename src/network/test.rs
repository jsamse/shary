#[cfg(test)]
use std::net::Ipv6Addr;

use super::*;

#[test]
fn broadcast_is_received() {
    let sender = UdpSocket::bind("0.0.0.0:17600").unwrap();
    sender.set_broadcast(true).unwrap();
    let receiver = UdpSocket::bind("0.0.0.0:17700").unwrap();
    receiver.set_broadcast(true).unwrap();
    sender
        .send_to("test".as_bytes(), "255.255.255.255:17700")
        .unwrap();
    let mut buf = vec![0; 10];
    let (size, addr) = receiver.recv_from(&mut buf).unwrap();
    let word = Vec::from(&buf[0..size]);
    println!(
        "{:?} {:?} {:?}",
        size,
        String::from_utf8(word).unwrap(),
        addr
    );
}

#[test]
fn ipv4_multicast() {
    let multicast_addr = ip_addr!(v4, "224.0.0.139");
    let first = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 17771)).unwrap();
    first
        .join_multicast_v4(&multicast_addr, &Ipv4Addr::UNSPECIFIED)
        .unwrap();

    let second = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 17772)).unwrap();
    second
        .join_multicast_v4(&multicast_addr, &Ipv4Addr::UNSPECIFIED)
        .unwrap();

    let mut buf = "test".as_bytes().to_vec();
    first.send_to(&buf, (multicast_addr, 17772)).unwrap();
    buf.fill(0);
    let (size, addr) = second.recv_from(&mut buf).unwrap();
    println!(
        "Received: {:?} {:?} {:?}",
        size,
        String::from_utf8((&buf[0..size]).to_vec()).unwrap(),
        addr
    );
}

#[test]
fn ipv6_multicast() {
    let multicast_addr = ip_addr!(v6, "ff02::139");
    assert!(multicast_addr.is_multicast());
    let first = UdpSocket::bind((Ipv6Addr::UNSPECIFIED, 17773)).unwrap();
    first.join_multicast_v6(&multicast_addr, 0).unwrap();

    let second = UdpSocket::bind((Ipv6Addr::UNSPECIFIED, 17774)).unwrap();
    second.join_multicast_v6(&multicast_addr, 0).unwrap();

    let mut buf = "test".as_bytes().to_vec();
    first.send_to(&buf, (multicast_addr, 17774)).unwrap();
    buf.fill(0);
    let (size, addr) = second.recv_from(&mut buf).unwrap();
    println!(
        "Received: {:?} {:?} {:?}",
        size,
        String::from_utf8((&buf[0..size]).to_vec()).unwrap(),
        addr
    );
}
