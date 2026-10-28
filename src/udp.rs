use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

use crate::model::UdpMeasurement;

const MAGIC_COOKIE: [u8; 4] = [0x21, 0x12, 0xA4, 0x42];
const BINDING_REQUEST_PREFIX: [u8; 8] = [0x00, 0x01, 0x00, 0x00, 0x21, 0x12, 0xA4, 0x42];

#[derive(Debug)]
enum StunAttemptError {
    Timeout,
    Protocol(String),
}

async fn resolve_target(target: &str, port: u16, timeout: Duration) -> Result<IpAddr, String> {
    if let Ok(ip) = target.parse::<IpAddr>() {
        return Ok(ip);
    }
    let lookup = tokio::time::timeout(timeout, tokio::net::lookup_host((target, port)))
        .await
        .map_err(|_| format!("DNS resolution failed: timed out after {timeout:?}"))?
        .map_err(|e| format!("DNS resolution failed: {e}"))?;
    let ips: Vec<IpAddr> = lookup.map(|address| address.ip()).collect();
    crate::pick_ip(&ips).ok_or_else(|| "DNS returned no addresses".to_string())
}

fn validate_stun_response(response: &[u8], transaction_id: &[u8]) -> Result<(), String> {
    if transaction_id.len() != 12 {
        return Err("STUN transaction ID must be exactly 12 bytes".to_string());
    }
    if response.len() < 20 {
        return Err(format!(
            "STUN message shorter than header: {} bytes",
            response.len()
        ));
    }
    if response[0] & 0b1100_0000 != 0 {
        return Err("invalid STUN message type bits".to_string());
    }
    if response[4..8] != MAGIC_COOKIE {
        return Err("invalid STUN magic cookie".to_string());
    }
    if response[0..2] != [0x01, 0x01] {
        return Err("response is not a STUN Binding Success Response".to_string());
    }
    let declared_len = usize::from(u16::from_be_bytes([response[2], response[3]]));
    if declared_len % 4 != 0 || response.len() != 20 + declared_len {
        return Err(format!(
            "STUN message length mismatch: declared {declared_len} attribute bytes, received {}",
            response.len().saturating_sub(20)
        ));
    }
    if &response[8..20] != transaction_id {
        return Err("STUN transaction ID does not match request".to_string());
    }

    let mut offset = 20;
    while offset < response.len() {
        if response.len() - offset < 4 {
            return Err("truncated STUN attribute header".to_string());
        }
        let attribute_len = usize::from(u16::from_be_bytes([
            response[offset + 2],
            response[offset + 3],
        ]));
        let padded_len = (attribute_len + 3) & !3;
        if offset + 4 + padded_len > response.len() {
            return Err("truncated STUN attribute value".to_string());
        }
        offset += 4 + padded_len;
    }
    Ok(())
}

async fn stun_attempt(addr: SocketAddr, timeout: Duration) -> Result<(), StunAttemptError> {
    let bind_addr = if addr.is_ipv4() {
        "0.0.0.0:0"
    } else {
        "[::]:0"
    };
    let socket = tokio::net::UdpSocket::bind(bind_addr)
        .await
        .map_err(|e| StunAttemptError::Protocol(format!("UDP socket bind failed: {e}")))?;
    socket
        .connect(addr)
        .await
        .map_err(|e| StunAttemptError::Protocol(format!("UDP socket connect failed: {e}")))?;

    let mut request = [0u8; 20];
    request[..8].copy_from_slice(&BINDING_REQUEST_PREFIX);
    rustls::crypto::ring::default_provider()
        .secure_random
        .fill(&mut request[8..20])
        .map_err(|e| {
            StunAttemptError::Protocol(format!("STUN transaction ID generation failed: {e:?}"))
        })?;
    socket
        .send(&request)
        .await
        .map_err(|e| StunAttemptError::Protocol(format!("UDP send failed: {e}")))?;

    let mut response = [0u8; 65_535];
    let response_len = tokio::time::timeout(timeout, socket.recv(&mut response))
        .await
        .map_err(|_| StunAttemptError::Timeout)?
        .map_err(|e| StunAttemptError::Protocol(format!("UDP receive failed: {e}")))?;
    validate_stun_response(&response[..response_len], &request[8..20])
        .map_err(StunAttemptError::Protocol)
}

fn endpoint_target(target: &str, port: u16) -> String {
    match target.parse::<IpAddr>() {
        Ok(ip) if ip.is_ipv6() => format!("[{ip}]:{port}"),
        _ => format!("{target}:{port}"),
    }
}

fn measurement(target: &str, port: u16, transport: &str) -> UdpMeasurement {
    UdpMeasurement {
        target: endpoint_target(target, port),
        resolved_ip: None,
        transport: transport.to_string(),
        port,
        elapsed_ms: 0,
        attempts: 0,
        successes: 0,
        status: "resolution_error".to_string(),
        failure_category: Some("resolution_error".to_string()),
        error: None,
    }
}

fn record_resolution_error(result: &mut UdpMeasurement, start: Instant, error: String) {
    result.error = Some(error);
    result.elapsed_ms = start.elapsed().as_millis() as u64;
}

pub async fn probe_stun(target: &str, port: u16, timeout: Duration, repeat: u32) -> UdpMeasurement {
    let start = Instant::now();
    let mut result = measurement(target, port, "udp/stun-binding");
    let ip = match resolve_target(target, port, timeout).await {
        Ok(ip) => ip,
        Err(error) => {
            record_resolution_error(&mut result, start, error);
            return result;
        }
    };
    result.resolved_ip = Some(ip.to_string());

    let mut errors = Vec::new();
    let mut timeouts = 0u32;
    for _ in 0..repeat.max(1) {
        result.attempts += 1;
        match stun_attempt(SocketAddr::new(ip, port), timeout).await {
            Ok(()) => result.successes += 1,
            Err(StunAttemptError::Timeout) => timeouts += 1,
            Err(StunAttemptError::Protocol(error)) => errors.push(error),
        }
    }

    if result.successes > 0 {
        result.status = "success".to_string();
        result.failure_category = None;
    } else if !errors.is_empty() {
        result.status = "protocol_error".to_string();
        result.failure_category = Some("protocol_error".to_string());
        result.error = Some(errors.join("; "));
    } else if timeouts > 0 {
        result.status = "timeout".to_string();
        result.failure_category = Some("timeout".to_string());
        result.error = Some(format!(
            "no valid STUN response within {timeout:?} ({timeouts}/{} attempts)",
            result.attempts
        ));
    } else {
        result.status = "protocol_error".to_string();
        result.failure_category = Some("protocol_error".to_string());
        result.error = Some("STUN probe did not produce a result".to_string());
    }
    result.elapsed_ms = start.elapsed().as_millis() as u64;
    result
}

pub async fn probe_quic(target: &str, port: u16, timeout: Duration, repeat: u32) -> UdpMeasurement {
    let start = Instant::now();
    let mut result = measurement(target, port, "udp/quic-h3");
    let ip = match resolve_target(target, port, timeout).await {
        Ok(ip) => ip,
        Err(error) => {
            record_resolution_error(&mut result, start, error);
            return result;
        }
    };
    result.resolved_ip = Some(ip.to_string());

    let mut errors = Vec::new();
    let mut timeouts = 0u32;
    for _ in 0..repeat.max(1) {
        result.attempts += 1;
        let probe = crate::h3::probe_port(ip, target, port, false, timeout).await;
        if probe.negotiated && probe.alpn.as_deref() == Some("h3") {
            result.successes += 1;
        } else if probe.failure_category.as_deref() == Some("timeout") {
            timeouts += 1;
        } else {
            errors.push(probe.note);
        }
    }
    if result.successes > 0 {
        result.status = "success".to_string();
        result.failure_category = None;
    } else if !errors.is_empty() {
        result.status = "protocol_error".to_string();
        result.failure_category = Some("protocol_error".to_string());
        result.error = Some(errors.join("; "));
    } else if timeouts > 0 {
        result.status = "timeout".to_string();
        result.failure_category = Some("timeout".to_string());
        result.error = Some(format!(
            "QUIC handshake timed out after {timeouts}/{} attempts",
            result.attempts
        ));
    } else {
        result.status = "protocol_error".to_string();
        result.failure_category = Some("protocol_error".to_string());
        result.error = Some("QUIC handshake failed".to_string());
    }
    result.elapsed_ms = start.elapsed().as_millis() as u64;
    result
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::time::Duration;

    use super::{UdpMeasurement, endpoint_target, probe_quic, probe_stun};

    #[derive(Clone, Copy)]
    enum Reply {
        Valid,
        ValidWithAttributes,
        WrongTransaction,
        MalformedLength,
        TruncatedAttribute,
        WrongCookie,
        WrongType,
        Short,
        Arbitrary,
    }

    async fn udp_server(reply: Reply, requests: usize) -> SocketAddr {
        let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = socket.local_addr().unwrap();
        tokio::spawn(async move {
            for _ in 0..requests {
                let mut request = [0u8; 64];
                let (len, peer) = socket.recv_from(&mut request).await.unwrap();
                assert_eq!(len, 20);
                assert_eq!(&request[..2], &[0x00, 0x01]);
                assert_eq!(&request[4..8], &[0x21, 0x12, 0xA4, 0x42]);

                let mut response = request[..20].to_vec();
                response[..2].copy_from_slice(&[0x01, 0x01]);
                match reply {
                    Reply::Valid => {}
                    Reply::ValidWithAttributes => {
                        // XOR-MAPPED-ADDRESS: attribute header + 8 value bytes.
                        response[2..4].copy_from_slice(&12u16.to_be_bytes());
                        response.extend_from_slice(&[
                            0x00, 0x20, 0x00, 0x08, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04,
                        ]);
                    }
                    Reply::WrongTransaction => response[19] ^= 1,
                    Reply::MalformedLength => response[2..4].copy_from_slice(&4u16.to_be_bytes()),
                    Reply::TruncatedAttribute => {
                        response[2..4].copy_from_slice(&4u16.to_be_bytes());
                        // Attribute header claims 8 value bytes but the datagram ends here.
                        response.extend_from_slice(&[0x00, 0x20, 0x00, 0x08]);
                    }
                    Reply::WrongCookie => response[4] ^= 1,
                    Reply::WrongType => response[1] = 0x02,
                    Reply::Short => response.truncate(2),
                    Reply::Arbitrary => response = vec![0xde; 20],
                }
                socket.send_to(&response, peer).await.unwrap();
            }
        });
        addr
    }

    async fn probe(addr: SocketAddr) -> UdpMeasurement {
        probe_stun(
            &addr.ip().to_string(),
            addr.port(),
            Duration::from_secs(1),
            1,
        )
        .await
    }

    #[tokio::test]
    async fn accepts_valid_binding_success_response() {
        let result = probe(udp_server(Reply::Valid, 1).await).await;
        assert_eq!(result.status, "success");
        assert_eq!(result.attempts, 1);
        assert_eq!(result.successes, 1);
        assert_eq!(result.transport, "udp/stun-binding");
        assert!(result.resolved_ip.is_some());
    }

    #[tokio::test]
    async fn accepts_valid_response_with_attributes() {
        let result = probe(udp_server(Reply::ValidWithAttributes, 1).await).await;
        assert_eq!(result.status, "success");
        assert_eq!(result.successes, 1);
    }

    #[tokio::test]
    async fn repeats_stun_transactions_and_counts_successes() {
        let addr = udp_server(Reply::Valid, 3).await;
        let result = probe_stun(
            &addr.ip().to_string(),
            addr.port(),
            Duration::from_secs(1),
            3,
        )
        .await;
        assert_eq!(result.status, "success");
        assert_eq!(result.attempts, 3);
        assert_eq!(result.successes, 3);
    }

    #[tokio::test]
    async fn valid_response_preserves_target_transport_and_timing_fields_in_json() {
        let result = probe(udp_server(Reply::Valid, 1).await).await;
        let value = serde_json::to_value(result).unwrap();
        assert_eq!(
            value["target"]
                .as_str()
                .unwrap()
                .rsplit_once(':')
                .unwrap()
                .1,
            value["port"].to_string()
        );
        assert_eq!(value["resolved_ip"], "127.0.0.1");
        assert_eq!(value["transport"], "udp/stun-binding");
        assert!(value["elapsed_ms"].is_number());
        assert_eq!(value["attempts"], 1);
        assert_eq!(value["successes"], 1);
        assert_eq!(value["status"], "success");
        assert!(value["failure_category"].is_null());
    }

    #[tokio::test]
    async fn rejects_mismatched_transaction_id() {
        let result = probe(udp_server(Reply::WrongTransaction, 1).await).await;
        assert_eq!(result.status, "protocol_error");
        assert_eq!(result.failure_category.as_deref(), Some("protocol_error"));
        assert!(result.error.as_deref().unwrap().contains("transaction ID"));
    }

    #[tokio::test]
    async fn rejects_malformed_declared_message_length() {
        let result = probe(udp_server(Reply::MalformedLength, 1).await).await;
        assert_eq!(result.status, "protocol_error");
        assert_eq!(result.failure_category.as_deref(), Some("protocol_error"));
        assert!(result.error.as_deref().unwrap().contains("length mismatch"));
    }

    #[tokio::test]
    async fn rejects_truncated_attribute() {
        let result = probe(udp_server(Reply::TruncatedAttribute, 1).await).await;
        assert_eq!(result.status, "protocol_error");
        assert!(
            result
                .error
                .as_deref()
                .unwrap()
                .contains("truncated STUN attribute")
        );
    }

    #[tokio::test]
    async fn rejects_wrong_cookie_type_and_short_packets() {
        for reply in [Reply::WrongCookie, Reply::WrongType, Reply::Short] {
            let result = probe(udp_server(reply, 1).await).await;
            assert_eq!(result.status, "protocol_error");
        }
    }

    #[tokio::test]
    async fn rejects_arbitrary_udp_payload() {
        let result = probe(udp_server(Reply::Arbitrary, 1).await).await;
        assert_eq!(result.status, "protocol_error");
        assert_eq!(result.failure_category.as_deref(), Some("protocol_error"));
        assert_eq!(result.successes, 0);
    }

    #[tokio::test]
    async fn classifies_no_response_as_timeout_only() {
        let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = socket.local_addr().unwrap();
        let result = probe_stun(
            &addr.ip().to_string(),
            addr.port(),
            Duration::from_millis(50),
            1,
        )
        .await;
        assert_eq!(result.status, "timeout");
        assert_eq!(result.attempts, 1);
        assert_eq!(result.successes, 0);
        assert_eq!(result.failure_category.as_deref(), Some("timeout"));
        assert!(
            result
                .error
                .as_deref()
                .unwrap()
                .contains("no valid STUN response")
        );
    }

    #[tokio::test]
    async fn reports_resolution_failure_before_attempting_udp() {
        let result = probe_stun("bad host", 3478, Duration::from_millis(50), 1).await;
        assert_eq!(result.status, "resolution_error");
        assert_eq!(result.failure_category.as_deref(), Some("resolution_error"));
        assert_eq!(result.attempts, 0);
        assert_eq!(result.successes, 0);
        assert_eq!(result.resolved_ip, None);
    }

    #[tokio::test]
    async fn quic_probe_reports_resolution_failure_separately() {
        let result = probe_quic("bad host", 443, Duration::from_millis(50), 1).await;
        assert_eq!(result.status, "resolution_error");
        assert_eq!(result.transport, "udp/quic-h3");
        assert_eq!(result.attempts, 0);
    }

    #[test]
    fn target_formatting_brackets_ipv6_only() {
        assert_eq!(endpoint_target("2001:db8::1", 3478), "[2001:db8::1]:3478");
        assert_eq!(
            endpoint_target("stun.example.test", 3478),
            "stun.example.test:3478"
        );
    }
}
