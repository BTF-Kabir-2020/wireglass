use std::net::IpAddr;

use crate::{dns, pick_ip};

pub async fn validate_url(raw: &str) -> Result<IpAddr, String> {
    let parsed = url::Url::parse(raw).map_err(|e| format!("invalid url: {e}"))?;
    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err("only http(s) URLs are allowed".to_string());
    }
    if parsed.host_str().is_none() {
        return Err("url has no host".to_string());
    }
    if parsed.username() != "" || parsed.password().is_some() {
        return Err("user info in URLs is not allowed".to_string());
    }
    let host = parsed.host_str().unwrap().to_string();
    let port = parsed.port_or_known_default().unwrap_or(443);

    // Pin the resolved address up front so DNS rebinding cannot swap targets
    // between validation and the connection step.
    let ip = if let Ok(lit) = host.parse::<IpAddr>() {
        lit
    } else {
        let outcome = dns::probe(&host).await.map_err(|e| e.to_string())?;
        pick_ip(&outcome.ips).ok_or_else(|| "no IP address to connect to".to_string())?
    };

    ensure_public(ip, port)?;
    Ok(ip)
}

fn ensure_public(ip: IpAddr, _port: u16) -> Result<(), String> {
    let is_unsafe = match ip {
        IpAddr::V4(v) => {
            v.is_loopback()
                || v.is_private()
                || v.is_link_local()
                || v.is_multicast()
                || v.is_broadcast()
                || v.is_unspecified()
                || (v.octets()[0] == 169 && v.octets()[1] == 254) // link-local + metadata
                || v.octets()[0] == 127
        }
        IpAddr::V6(v) => {
            v.is_loopback()
                || v.is_unique_local()
                || v.is_multicast()
                || v.is_unspecified()
                || v.is_unicast_link_local()
        }
    };
    if is_unsafe {
        return Err(format!("target resolves to a non-public address: {ip}"));
    }
    Ok(())
}

pub fn validate_ip(ip: IpAddr) -> Result<(), String> {
    ensure_public(ip, 0)
}
