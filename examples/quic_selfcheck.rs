// Self-check: proves the QUIC/HTTP-3 client path works end-to-end on loopback,
// independent of whether the local network allows outbound UDP/443.
//
//   cargo run --example quic_selfcheck

use std::sync::Arc;
use std::time::Duration;

use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let issued = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])?;
    let cert_der = CertificateDer::from(issued.cert.der().to_vec());
    let key_der =
        PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(issued.signing_key.serialize_der()));

    let mut server_cfg = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der.clone()], key_der)?;
    server_cfg.alpn_protocols = vec![b"h3".to_vec()];
    let qsc = quinn::crypto::rustls::QuicServerConfig::try_from(server_cfg)?;
    let sc = quinn::ServerConfig::with_crypto(Arc::new(qsc));

    let server = quinn::Endpoint::server(sc, "127.0.0.1:0".parse()?)?;
    let addr = server.local_addr()?;

    tokio::spawn(async move {
        if let Some(incoming) = server.accept().await {
            match incoming.await {
                Ok(conn) => {
                    let alpn = conn
                        .handshake_data()
                        .and_then(|d| d.downcast::<quinn::crypto::rustls::HandshakeData>().ok())
                        .and_then(|h| h.protocol)
                        .map(|p| String::from_utf8_lossy(&p).into_owned());
                    println!("[server] accepted QUIC connection, alpn={alpn:?}");
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    conn.close(0u32.into(), b"bye");
                }
                Err(e) => eprintln!("[server] accept failed: {e}"),
            }
        }
    });

    let mut roots = rustls::RootCertStore::empty();
    roots.add(cert_der)?;
    let mut cfg = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    cfg.alpn_protocols = vec![b"h3".to_vec()];
    let qcc = quinn::crypto::rustls::QuicClientConfig::try_from(cfg)?;
    let client_cfg = quinn::ClientConfig::new(Arc::new(qcc));

    let mut endpoint = quinn::Endpoint::client("127.0.0.1:0".parse()?)?;
    endpoint.set_default_client_config(client_cfg);

    let conn = tokio::time::timeout(Duration::from_secs(5), endpoint.connect(addr, "localhost")?)
        .await
        .map_err(|_| anyhow::anyhow!("client QUIC handshake timed out"))??;

    let alpn = conn
        .handshake_data()
        .and_then(|d| d.downcast::<quinn::crypto::rustls::HandshakeData>().ok())
        .and_then(|h| h.protocol)
        .map(|p| String::from_utf8_lossy(&p).into_owned());

    println!("[client] QUIC handshake OK, negotiated alpn={alpn:?}");
    endpoint.close(0u32.into(), b"done");
    endpoint.wait_idle().await;
    Ok(())
}
