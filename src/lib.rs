pub mod card;
pub mod dns;
pub mod h3;
pub mod http;
mod model;
pub mod net;
pub mod ports;
pub mod report;
pub mod score;
pub mod ssrf;
pub mod tls;
pub mod udp;

use std::net::IpAddr;
use std::time::Duration;

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use url::Url;

pub use model::{DotData, PortProbe, Report, Stage, UdpMeasurement, UdpReport};

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

/// Progress hook: stage name just starting (`dns`/`tcp`/`tls`/`http`/`http3`).
pub type StageHook = Box<dyn FnMut(&str) + Send>;

pub async fn run(url: &str, timeout_secs: u64, repeat: u32) -> Report {
    run_with_progress(url, timeout_secs, repeat, None, None).await
}

/// Same as [`run`], with optional live stage hook and cooperative cancel.
/// When `cancel` is set between stages, remaining stages stay `not_attempted`
/// with reason `cancelled` and the partial report is returned.
pub async fn run_with_progress(
    url: &str,
    timeout_secs: u64,
    repeat: u32,
    mut on_stage: Option<StageHook>,
    cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
) -> Report {
    use std::sync::atomic::Ordering;

    let cancelled = || {
        cancel
            .as_ref()
            .map(|c| c.load(Ordering::SeqCst))
            .unwrap_or(false)
    };
    let mark = |hook: &mut Option<StageHook>, name: &str| {
        if let Some(h) = hook.as_mut() {
            h(name);
        }
    };

    let timeout = Duration::from_secs(timeout_secs);
    let ran_at_utc = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_default();

    let mut report = Box::new(Report {
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
        dot: not_attempted(),
        score: None,
        ports: Vec::new(),
        udp_measurements: Vec::new(),
    });

    let parsed = match Url::parse(url) {
        Ok(u) => u,
        Err(e) => {
            report.dns = Stage::Failed {
                error: format!("invalid url: {e}"),
            };
            return *report;
        }
    };
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        report.dns = Stage::Failed {
            error: format!("unsupported scheme: {} (only http/https)", parsed.scheme()),
        };
        return *report;
    }
    if parsed.username() != "" || parsed.password().is_some() {
        report.dns = Stage::Failed {
            error: "user info in URLs is not allowed".to_string(),
        };
        return *report;
    }
    let Some(host) = parsed.host_str().map(str::to_string) else {
        report.dns = Stage::Failed {
            error: "url has no host".to_string(),
        };
        return *report;
    };
    let port = parsed.port_or_known_default().unwrap_or(443);
    let path = match parsed.path() {
        "" => "/".to_string(),
        p => p.to_string(),
    };
    report.host = host.clone();
    report.port = port;
    drop(parsed);

    if cancelled() {
        report.dns = Stage::NotAttempted {
            reason: "cancelled".into(),
        };
        return *report;
    }
    mark(&mut on_stage, "dns");
    let outcome = match Box::pin(dns::probe(&host)).await {
        Ok(o) => o,
        Err(e) => {
            report.dns = Stage::Failed {
                error: e.to_string(),
            };
            return *report;
        }
    };
    let ip = pick_ip(&outcome.ips);
    if let Some(candidate) = ip {
        if let Err(e) = ssrf::validate_ip(candidate) {
            report.tcp = Stage::Failed {
                error: format!("ssrf guard: {e}"),
            };
            return *report;
        }
    }
    report.resolved_ip = ip.map(|i| i.to_string());
    report.dns = Stage::Ok(outcome.data);

    let Some(ip) = ip else {
        report.tcp = Stage::Failed {
            error: "no IP address to connect to".to_string(),
        };
        return *report;
    };

    if cancelled() {
        report.tcp = Stage::NotAttempted {
            reason: "cancelled".into(),
        };
        return *report;
    }
    mark(&mut on_stage, "tcp");

    let repeat = repeat.max(1);
    let mut connect_samples: Vec<u64> = Vec::with_capacity(repeat as usize);
    let mut handshake_samples: Vec<u64> = Vec::with_capacity(repeat as usize);
    let mut kept: Option<(tls::ClientTlsStream, model::TcpData, model::TlsData)> = None;
    let mut tls_failed: Option<String> = None;

    for _ in 0..repeat {
        if cancelled() {
            report.tcp = Stage::NotAttempted {
                reason: "cancelled".into(),
            };
            return *report;
        }
        let (stream, tcp_data) = match Box::pin(net::connect(ip, port, timeout)).await {
            Ok(v) => v,
            Err(e) => {
                report.tcp = Stage::Failed {
                    error: e.to_string(),
                };
                return *report;
            }
        };
        if cancelled() {
            report.tcp = Stage::Ok(tcp_data);
            report.tls = Stage::NotAttempted {
                reason: "cancelled".into(),
            };
            return *report;
        }
        mark(&mut on_stage, "tls");
        match Box::pin(tls::connect(stream, &host, timeout)).await {
            Ok(v) => {
                let (tls_stream, tls_data) = v;
                connect_samples.push(tcp_data.connect_ms);
                handshake_samples.push(tls_data.handshake_ms);
                kept = Some((tls_stream, tcp_data, tls_data));
            }
            Err(e) => {
                // Commit the TCP result we already have, record TLS failure,
                // and continue to UDP/DNS port probes + scoring instead of
                // aborting the whole report.
                report.tcp = Stage::Ok(tcp_data);
                report.tls = Stage::Failed {
                    error: e.to_string(),
                };
                tls_failed = Some(e.to_string());
                break;
            }
        }
    }

    let tls_ok = tls_failed.is_none();

    // For DNS targets (port 53) TLS/HTTP/HTTP3 are not applicable; skip them
    // to avoid a misleading 10s TLS timeout and keep the focus on UDP DNS.
    let dns_port = port == 53;

    let alt_svc = if dns_port {
        report.tls = Stage::NotAttempted {
            reason: "not applicable for DNS port".into(),
        };
        report.http = Stage::NotAttempted {
            reason: "not applicable for DNS port".into(),
        };
        report.http3 = Stage::NotAttempted {
            reason: "not applicable for DNS port".into(),
        };
        None
    } else if tls_ok {
        let Some((tls_stream, mut tcp_data, mut tls_data)) = kept else {
            report.tcp = Stage::Failed {
                error: "no connection attempt was made".to_string(),
            };
            return *report;
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

        if cancelled() {
            report.http = Stage::NotAttempted {
                reason: "cancelled".into(),
            };
            None
        } else {
            mark(&mut on_stage, "http");
            match Box::pin(http::get(tls_stream, &host, &path, alpn_is_h2, timeout)).await {
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
            }
        }
    } else {
        report.http = Stage::NotAttempted {
            reason: "not run".into(),
        };
        report.http3 = Stage::NotAttempted {
            reason: "not run".into(),
        };
        None
    };

    if !dns_port && tls_ok && !cancelled() {
        if cancelled() {
            report.http3 = Stage::NotAttempted {
                reason: "cancelled".into(),
            };
        } else {
            mark(&mut on_stage, "http3");
            let advertised = alt_svc
                .as_deref()
                .map(|s| s.to_ascii_lowercase().contains("h3"))
                .unwrap_or(false);
            report.http3 =
                Stage::Ok(Box::pin(h3::probe_port(ip, &host, port, advertised, timeout)).await);
        }
    }

    if !cancelled() {
        mark(&mut on_stage, "ports");
        let port_results = Box::pin(ports::probe(&host, timeout)).await;
        report.ports = port_results;
    } else {
        report.ports = Vec::new();
    }

    if !cancelled() {
        mark(&mut on_stage, "dot");
        let dot_result = tokio::time::timeout(timeout, async {
            let stream = tokio::net::TcpStream::connect((ip, 853))
                .await
                .map_err(|e| format!("tcp connect failed: {e}"))?;
            let (_, data) = tls::connect(stream, &host, timeout)
                .await
                .map_err(|e| e.to_string())?;
            Ok(data)
        })
        .await;
        report.dot = match dot_result {
            Ok(Ok(data)) => Stage::Ok(DotData {
                port: 853,
                alpn: data.alpn,
                protocol_version: data.protocol_version,
                cipher_suite: data.cipher_suite,
                cert_subject: data.cert_subject,
                cert_issuer: data.cert_issuer,
                cert_not_after: data.cert_not_after,
                handshake_ms: data.handshake_ms,
                note: String::new(),
            }),
            Ok(Err(e)) => Stage::Failed { error: e },
            Err(_) => Stage::Failed {
                error: format!("dot timed out after {timeout:?}"),
            },
        };
    } else {
        report.dot = Stage::NotAttempted {
            reason: "cancelled".into(),
        };
    }

    report.score = Some(score::compute(&report));
    *report
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
