use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use rustls::pki_types::ServerName;
use time::format_description::well_known::Rfc3339;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;

use crate::model::TlsData;

pub type ClientTlsStream = TlsStream<TcpStream>;

pub fn root_store() -> rustls::RootCertStore {
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    roots
}

pub async fn connect(
    stream: TcpStream,
    host: &str,
    timeout: Duration,
) -> Result<(ClientTlsStream, TlsData)> {
    let mut cfg = rustls::ClientConfig::builder()
        .with_root_certificates(root_store())
        .with_no_client_auth();
    cfg.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    let connector = TlsConnector::from(Arc::new(cfg));
    let server_name = ServerName::try_from(host.to_string())
        .map_err(|e| anyhow!("invalid SNI/server name {host}: {e}"))?;

    let start = Instant::now();
    let tls = tokio::time::timeout(timeout, connector.connect(server_name, stream))
        .await
        .map_err(|_| anyhow!("tls handshake timed out after {timeout:?}"))?
        .context("tls handshake failed")?;
    let handshake_ms = start.elapsed().as_millis() as u64;

    let (_, conn) = tls.get_ref();
    let alpn = conn
        .alpn_protocol()
        .map(|p| String::from_utf8_lossy(p).into_owned());
    let protocol_version = conn
        .protocol_version()
        .map(|v| format!("{v:?}"))
        .unwrap_or_default();
    let cipher_suite = conn
        .negotiated_cipher_suite()
        .map(|c| format!("{:?}", c.suite()))
        .unwrap_or_default();
    let (cert_subject, cert_issuer, cert_not_after) = cert_info(conn);

    Ok((
        tls,
        TlsData {
            alpn,
            protocol_version,
            cipher_suite,
            cert_subject,
            cert_issuer,
            cert_not_after,
            handshake_ms,
            ..Default::default()
        },
    ))
}

fn cert_info(conn: &rustls::ClientConnection) -> (String, String, String) {
    let Some(cert) = conn.peer_certificates().and_then(|c| c.first()) else {
        return (String::new(), String::new(), String::new());
    };
    match x509_parser::parse_x509_certificate(cert.as_ref()) {
        Ok((_, c)) => {
            let not_after = c
                .validity()
                .not_after
                .to_datetime()
                .format(&Rfc3339)
                .unwrap_or_default();
            (c.subject().to_string(), c.issuer().to_string(), not_after)
        }
        Err(_) => (String::new(), String::new(), String::new()),
    }
}
