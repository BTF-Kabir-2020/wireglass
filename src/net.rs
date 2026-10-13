use std::net::IpAddr;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use tokio::net::TcpStream;

use crate::model::TcpData;

pub async fn connect(ip: IpAddr, port: u16, timeout: Duration) -> Result<(TcpStream, TcpData)> {
    let start = Instant::now();
    let stream = tokio::time::timeout(timeout, TcpStream::connect((ip, port)))
        .await
        .map_err(|_| anyhow!("tcp connect timed out after {timeout:?}"))?
        .map_err(|e| anyhow!("tcp connect to {ip}:{port} failed: {e}"))?;
    let connect_ms = start.elapsed().as_millis() as u64;
    stream.set_nodelay(true).ok();
    Ok((
        stream,
        TcpData {
            addr: format!("{ip}:{port}"),
            connect_ms,
            ..Default::default()
        },
    ))
}
