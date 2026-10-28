use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};

use crate::model::Http3Data;
use crate::tls::root_store;

pub async fn probe(ip: IpAddr, host: &str, advertised: bool, timeout: Duration) -> Http3Data {
    probe_port(ip, host, 443, advertised, timeout).await
}

pub async fn probe_port(
    ip: IpAddr,
    host: &str,
    port: u16,
    advertised: bool,
    timeout: Duration,
) -> Http3Data {
    let start = Instant::now();
    let mut data = Http3Data {
        advertised,
        ..Default::default()
    };

    match try_handshake(ip, host, port, timeout).await {
        Ok(Some(alpn)) if alpn == "h3" => {
            data.negotiated = true;
            data.alpn = Some(alpn);
            data.resolved_ip = Some(ip.to_string());
            data.note =
                "real QUIC handshake completed; full HTTP/3 request not implemented in skeleton"
                    .to_string();
            data.failure_category = None;
        }
        Ok(alpn) => {
            data.alpn = alpn;
            data.resolved_ip = Some(ip.to_string());
            data.failure_category = Some("protocol_error".to_string());
            data.note = format!(
                "QUIC connected but negotiated unexpected ALPN: {:?}",
                data.alpn
            );
        }
        Err(e) => {
            data.resolved_ip = Some(ip.to_string());
            data.fallback_used = advertised;
            data.failure_category = Some(
                if e.to_string().contains("quic handshake timed out") {
                    "timeout"
                } else {
                    "protocol_error"
                }
                .to_string(),
            );
            data.note = if advertised {
                format!("Alt-Svc advertised h3 but QUIC handshake failed: {e}")
            } else {
                format!("QUIC handshake failed (h3 was not advertised): {e}")
            };
        }
    }

    data.duration_ms = start.elapsed().as_millis() as u64;
    data
}

async fn try_handshake(
    ip: IpAddr,
    host: &str,
    port: u16,
    timeout: Duration,
) -> Result<Option<String>> {
    let mut cfg = rustls::ClientConfig::builder()
        .with_root_certificates(root_store())
        .with_no_client_auth();
    cfg.alpn_protocols = vec![b"h3".to_vec()];

    let qcc = quinn::crypto::rustls::QuicClientConfig::try_from(cfg)
        .map_err(|e| anyhow!("quic client config: {e}"))?;
    let client_cfg = quinn::ClientConfig::new(Arc::new(qcc));

    let mut endpoint = quinn::Endpoint::client("0.0.0.0:0".parse().unwrap())
        .map_err(|e| anyhow!("udp endpoint bind failed: {e}"))?;
    endpoint.set_default_client_config(client_cfg);

    let addr = SocketAddr::new(ip, port);
    let connecting = endpoint
        .connect(addr, host)
        .map_err(|e| anyhow!("quic connect init failed: {e}"))?;
    let conn = tokio::time::timeout(timeout, connecting)
        .await
        .map_err(|_| anyhow!("quic handshake timed out after {timeout:?}"))?
        .map_err(|e| anyhow!("quic handshake failed: {e}"))?;

    let alpn = conn
        .handshake_data()
        .and_then(|d| d.downcast::<quinn::crypto::rustls::HandshakeData>().ok())
        .and_then(|h| h.protocol)
        .map(|p| String::from_utf8_lossy(&p).into_owned());

    conn.close(0u32.into(), b"done");
    endpoint.wait_idle().await;
    Ok(alpn)
}
