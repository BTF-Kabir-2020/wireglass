pub mod card;
pub mod dns;
pub mod h3;
pub mod http;
mod model;
pub mod net;
pub mod report;
pub mod score;
pub mod ssrf;
pub mod tls;

use std::net::IpAddr;
use std::time::Duration;

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use url::Url;

pub use model::{Report, Stage};

pub fn pick_ip(ips: &[IpAddr]) -> Option<IpAddr> {
    ips.iter()
        .copied()
        .find(IpAddr::is_ipv4)
        .or_else(|| ips.first().copied())
}

fn not_attempted<T>() -> Stage<T> {
    Stage::NotAttempted {
        reason: "not run".to_string(),
    }
}

pub async fn run(url: &str, timeout_secs: u64, repeat: u32) -> Report {
    let timeout = Duration::from_secs(timeout_secs);
    let ran_at_utc = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_default();

    let mut report = Report {
        tool: "wireglass".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        target: url.to_string(),
        host: String::new(),
        port: 443,
        resolved_ip: None,
        ran_at_utc,
        dns: not_attempted(),
        tcp: not_attempted(),
        tls: not_attempted(),
        http: not_attempted(),
        http3: not_attempted(),
        score: None,
    };

    let parsed = match Url::parse(url) {
        Ok(u) => u,
        Err(e) => {
            report.dns = Stage::Failed {
                error: format!("invalid url: {e}"),
            };
            return report;
        }
    };
    let Some(host) = parsed.host_str().map(str::to_string) else {
        report.dns = Stage::Failed {
            error: "url has no host".to_string(),
        };
        return report;
    };
    let port = parsed.port_or_known_default().unwrap_or(443);
    let path = match parsed.path() {
        "" => "/",
        p => p,
    };
    report.host = host.clone();
    report.port = port;

    let outcome = match dns::probe(&host).await {
        Ok(o) => o,
        Err(e) => {
            report.dns = Stage::Failed {
                error: e.to_string(),
            };
            return report;
        }
    };
    let ip = pick_ip(&outcome.ips);
    if let Some(candidate) = ip {
        if let Err(e) = ssrf::validate_ip(candidate) {
            report.tcp = Stage::Failed {
                error: format!("ssrf guard: {e}"),
            };
            return report;
        }
    }
    report.resolved_ip = ip.map(|i| i.to_string());
    report.dns = Stage::Ok(outcome.data);

    let Some(ip) = ip else {
        report.tcp = Stage::Failed {
            error: "no IP address to connect to".to_string(),
        };
        return report;
    };

    let repeat = repeat.max(1);
    let mut connect_samples: Vec<u64> = Vec::with_capacity(repeat as usize);
    let mut handshake_samples: Vec<u64> = Vec::with_capacity(repeat as usize);
    let mut kept: Option<(tls::ClientTlsStream, model::TcpData, model::TlsData)> = None;

    for _ in 0..repeat {
        let (stream, tcp_data) = match net::connect(ip, port, timeout).await {
            Ok(v) => v,
            Err(e) => {
                report.tcp = Stage::Failed {
                    error: e.to_string(),
                };
                return report;
            }
        };
        let (tls_stream, tls_data) = match tls::connect(stream, &host, timeout).await {
            Ok(v) => v,
            Err(e) => {
                report.tls = Stage::Failed {
                    error: e.to_string(),
                };
                return report;
            }
        };
        connect_samples.push(tcp_data.connect_ms);
        handshake_samples.push(tls_data.handshake_ms);
        kept = Some((tls_stream, tcp_data, tls_data));
    }

    let Some((tls_stream, mut tcp_data, mut tls_data)) = kept else {
        report.tcp = Stage::Failed {
            error: "no connection attempt was made".to_string(),
        };
        return report;
    };
    tcp_data.connect_ms = *connect_samples.first().unwrap_or(&tcp_data.connect_ms);
    tls_data.handshake_ms = *handshake_samples.first().unwrap_or(&tls_data.handshake_ms);
    if repeat > 1 {
        tcp_data.connect_ms_samples = connect_samples;
        tls_data.handshake_ms_samples = handshake_samples;
    }

    let alpn_is_h2 = tls_data.alpn.as_deref() == Some("h2");
    report.tcp = Stage::Ok(tcp_data);
    report.tls = Stage::Ok(tls_data);

    let alt_svc = match http::get(tls_stream, &host, path, alpn_is_h2, timeout).await {
        Ok(d) => {
            let alt = d.alt_svc.clone();
            report.http = Stage::Ok(d);
            alt
        }
        Err(e) => {
            report.http = Stage::Failed {
                error: e.to_string(),
            };
            None
        }
    };

    let advertised = alt_svc
        .as_deref()
        .map(|s| s.to_ascii_lowercase().contains("h3"))
        .unwrap_or(false);
    report.http3 = Stage::Ok(h3::probe(ip, &host, advertised, timeout).await);

    report.score = Some(score::compute(&report));
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn pick_ip_prefers_ipv4() {
        let ips = vec![
            IpAddr::V6(Ipv6Addr::LOCALHOST),
            IpAddr::V4(Ipv4Addr::LOCALHOST),
        ];
        assert_eq!(pick_ip(&ips), Some(IpAddr::V4(Ipv4Addr::LOCALHOST)));
    }

    #[test]
    fn pick_ip_falls_back_to_first() {
        let ips = vec![IpAddr::V6(Ipv6Addr::LOCALHOST)];
        assert_eq!(pick_ip(&ips), Some(IpAddr::V6(Ipv6Addr::LOCALHOST)));
    }

    #[test]
    fn pick_ip_none_when_empty() {
        assert_eq!(pick_ip(&[]), None);
    }
}
