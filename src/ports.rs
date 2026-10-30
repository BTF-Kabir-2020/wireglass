use crate::model::PortProbe;
use std::net::IpAddr;
use std::time::{Duration, Instant};

pub const PORTS: [u16; 6] = [80, 443, 8080, 53, 853, 5353];

pub async fn probe(host: &str, timeout: Duration) -> Vec<PortProbe> {
    // Cap per-port timeout so a single closed port cannot stall the whole scan.
    let per_port = timeout.min(Duration::from_secs(3));
    let resolved_ip = if let Ok(ip) = host.parse::<IpAddr>() {
        Some(ip)
    } else {
        match tokio::time::timeout(per_port, tokio::net::lookup_host((host, 443))).await {
            Ok(Ok(mut addrs)) => addrs.next().map(|a| a.ip()),
            _ => None,
        }
    };

    let Some(ip) = resolved_ip else {
        // DNS failed: report honestly instead of probing 0.0.0.0.
        let mut out = Vec::new();
        for &port in &PORTS {
            out.push(PortProbe {
                port,
                protocol: "tcp".to_string(),
                status: "dns resolution failed".to_string(),
                ms: 0,
                note: format!("could not resolve host {host}"),
            });
        }
        return out;
    };

    let mut out = Vec::new();
    for &port in &PORTS {
        let start = Instant::now();
        let (status, protocol) = match port {
            53 => udp_dns_probe(ip, port, per_port).await,
            5353 => udp_mdns_probe(ip, port, per_port).await,
            _ => {
                let addr = (ip, port);
                let s = match tokio::time::timeout(per_port, tokio::net::TcpStream::connect(addr))
                    .await
                {
                    Ok(Ok(_)) => "open (tcp)".to_string(),
                    Ok(Err(e)) => format!("closed: {e}"),
                    Err(_) => "timeout (tcp)".to_string(),
                };
                (s, "tcp")
            }
        };
        out.push(PortProbe {
            port,
            protocol: protocol.to_string(),
            status,
            ms: start.elapsed().as_millis() as u64,
            note: String::new(),
        });
    }
    out
}

async fn udp_dns_probe(ip: IpAddr, port: u16, timeout: Duration) -> (String, &'static str) {
    let bind_addr = if ip.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" };
    let socket = match tokio::net::UdpSocket::bind(bind_addr).await {
        Ok(s) => s,
        Err(e) => return (format!("udp bind error: {e}"), "udp"),
    };
    if let Err(e) = socket.connect((ip, port)).await {
        return (format!("udp connect error: {e}"), "udp");
    }
    let query: [u8; 29] = [
        0x12, 0x34, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, b'e', b'x',
        b'a', b'm', b'p', b'l', b'e', 0x03, b'c', b'o', b'm', 0x00, 0x00, 0x01, 0x00, 0x01,
    ];
    if let Err(e) = socket.send(&query).await {
        return (format!("udp send error: {e}"), "udp");
    }
    let mut buf = [0u8; 512];
    match tokio::time::timeout(timeout, socket.recv(&mut buf)).await {
        Ok(Ok(n)) if n >= 12 && (buf[2] & 0x80) != 0 => ("open (udp dns)".to_string(), "udp"),
        Ok(Ok(n)) => (format!("udp response ({} bytes)", n), "udp"),
        Ok(Err(e)) => (format!("udp recv error: {e}"), "udp"),
        Err(_) => ("timeout (udp dns)".to_string(), "udp"),
    }
}

async fn udp_mdns_probe(ip: IpAddr, port: u16, timeout: Duration) -> (String, &'static str) {
    let bind_addr = if ip.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" };
    let socket = match tokio::net::UdpSocket::bind(bind_addr).await {
        Ok(s) => s,
        Err(e) => return (format!("udp bind error: {e}"), "udp"),
    };
    if !ip.is_ipv4() {
        return ("mdns: ipv6 not supported".to_string(), "udp");
    }
    let mcast = std::net::Ipv4Addr::new(224, 0, 0, 251);
    let query: &[u8] = &[
        0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x09, b'_', b's',
        b'e', b'r', b'v', b'i', b'c', b'e', b's', 0x07, b'_', b'd', b'n', b's', b'-', b's', b'd',
        0x04, b'_', b'u', b'd', b'p', 0x05, b'l', b'o', b'c', b'a', b'l', 0x00, 0x00, 0x0C, 0x00,
        0x01,
    ];
    let dest = std::net::SocketAddr::new(std::net::IpAddr::V4(mcast), port);
    if let Err(e) = socket.send_to(query, dest).await {
        return (format!("mdns send error: {e}"), "udp");
    }
    let mut buf = [0u8; 1500];
    match tokio::time::timeout(timeout, socket.recv_from(&mut buf)).await {
        Ok(Ok((n, _))) if n > 12 => ("open (mdns response)".to_string(), "udp"),
        Ok(Ok((n, _))) => (format!("mdns response ({} bytes)", n), "udp"),
        Ok(Err(e)) => (format!("mdns recv error: {e}"), "udp"),
        Err(_) => ("timeout (mdns)".to_string(), "udp"),
    }
}
