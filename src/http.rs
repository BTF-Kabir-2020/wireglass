use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::model::HttpData;
use crate::tls::ClientTlsStream;
use std::collections::BTreeMap;

const SECURITY_HEADERS: &[&str] = &[
    "strict-transport-security",
    "content-security-policy",
    "x-frame-options",
    "x-content-type-options",
    "referrer-policy",
    "permissions-policy",
    "cross-origin-opener-policy",
    "cross-origin-resource-policy",
    "cross-origin-embedder-policy",
];

fn secure_headers(headers: &http::HeaderMap) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for name in SECURITY_HEADERS {
        if let Some(v) = headers.get(*name) {
            if let Ok(s) = v.to_str() {
                out.insert((*name).to_string(), s.to_string());
            }
        }
    }
    out
}

pub async fn get(
    stream: ClientTlsStream,
    host: &str,
    path: &str,
    alpn_is_h2: bool,
    timeout: Duration,
) -> Result<HttpData> {
    if alpn_is_h2 {
        get_h2(stream, host, path, timeout).await
    } else {
        get_h1(stream, host, path, timeout).await
    }
}

fn header(headers: &http::HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

async fn get_h2(
    stream: ClientTlsStream,
    host: &str,
    path: &str,
    timeout: Duration,
) -> Result<HttpData> {
    let (mut send_req, connection) = tokio::time::timeout(timeout, h2::client::handshake(stream))
        .await
        .map_err(|_| anyhow!("http/2 handshake timed out after {timeout:?}"))?
        .context("http/2 handshake failed")?;
    tokio::spawn(async move {
        let _ = connection.await;
    });

    let uri = format!("https://{host}{path}");
    let req = http::Request::builder()
        .method("GET")
        .uri(&uri)
        .header("user-agent", "wireglass/0.1")
        .header("accept", "*/*")
        .body(())?;

    let start = Instant::now();
    let (resp_fut, _send_body) = send_req.send_request(req, true)?;
    let resp = tokio::time::timeout(timeout, resp_fut)
        .await
        .map_err(|_| anyhow!("http/2 response timed out"))?
        .context("http/2 request failed")?;

    let status = resp.status().as_u16();
    let server = header(resp.headers(), "server");
    let alt_svc = header(resp.headers(), "alt-svc");
    let headers = secure_headers(resp.headers());

    let mut body = resp.into_body();
    let mut body_bytes = 0usize;
    while let Ok(Some(Ok(chunk))) = tokio::time::timeout(timeout, body.data()).await {
        body_bytes += chunk.len();
        let _ = body.flow_control().release_capacity(chunk.len());
        if body_bytes > 128 * 1024 {
            break;
        }
    }

    Ok(HttpData {
        version: "2".to_string(),
        http_status: Some(status),
        server,
        alt_svc,
        body_bytes,
        duration_ms: start.elapsed().as_millis() as u64,
        headers,
    })
}

async fn get_h1(
    mut stream: ClientTlsStream,
    host: &str,
    path: &str,
    timeout: Duration,
) -> Result<HttpData> {
    let req = format!(
        "GET {path} HTTP/1.1\r\nHost: {host}\r\nUser-Agent: wireglass/0.1\r\nAccept: */*\r\nConnection: close\r\n\r\n"
    );
    let start = Instant::now();
    tokio::time::timeout(timeout, stream.write_all(req.as_bytes()))
        .await
        .map_err(|_| anyhow!("http/1.1 write timed out"))?
        .context("http/1.1 write failed")?;

    let mut buf = Vec::new();
    let mut tmp = [0u8; 8192];
    loop {
        let n = match tokio::time::timeout(timeout, stream.read(&mut tmp)).await {
            Ok(Ok(n)) => n,
            Ok(Err(e)) => return Err(anyhow!("http/1.1 read failed: {e}")),
            Err(_) => break,
        };
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if buf.len() > 128 * 1024 {
            break;
        }
    }

    let text = String::from_utf8_lossy(&buf);
    let status_line = text.lines().next().unwrap_or("");
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok());

    let mut server = None;
    let mut alt_svc = None;
    let mut headers = BTreeMap::new();
    for line in text.lines() {
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim().to_ascii_lowercase();
            let value = v.trim().to_string();
            match key.as_str() {
                "server" => server = Some(value.clone()),
                "alt-svc" => alt_svc = Some(value.clone()),
                _ => {}
            }
            if SECURITY_HEADERS.contains(&key.as_str()) {
                headers.insert(key, value);
            }
        }
    }

    let body_bytes = if let Some(pos) = buf.windows(4).position(|w| w == b"\\r\\n\\r\\n") {
        buf.len().saturating_sub(pos + 4)
    } else {
        0
    };

    Ok(HttpData {
        version: "1.1".to_string(),
        http_status: status,
        server,
        alt_svc,
        body_bytes,
        duration_ms: start.elapsed().as_millis() as u64,
        headers,
    })
}
