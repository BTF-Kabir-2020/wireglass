use crate::model::{Report, Stage};

/// Escape text for safe inclusion in HTML text/attribute context.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

fn badge(status: &str) -> String {
    let (cls, label) = match status {
        "ok" => ("ok", "ok"),
        "failed" => ("fail", "failed"),
        _ => ("skip", "not attempted"),
    };
    format!("<span class=\"badge {cls}\">{label}</span>")
}

fn kv(k: &str, v: &str) -> String {
    format!(
        "<div class=\"kv\"><span class=\"k\">{}</span><span class=\"v\">{}</span></div>",
        esc(k),
        esc(v)
    )
}

fn list(title: &str, items: &[String]) -> String {
    if items.is_empty() {
        return kv(title, "—");
    }
    let lis: Vec<String> = items
        .iter()
        .map(|i| format!("<li>{}</li>", esc(i)))
        .collect();
    format!(
        "<div class=\"kv\"><span class=\"k\">{}</span><span class=\"v\"><ul>{}</ul></span></div>",
        esc(title),
        lis.join("")
    )
}

fn stage_card<T: RenderStage>(title: &str, stage: &Stage<T>) -> String {
    let status = stage.status();
    let body = match stage {
        Stage::Ok(v) => v.render_rows(),
        Stage::Failed { error } => kv("error", error),
        Stage::NotAttempted { reason } => kv("reason", reason),
    };
    format!(
        "<section class=\"card {status}\">\
           <header><h2>{}</h2>{}</header>\
           <div class=\"rows\">{body}</div>\
         </section>",
        esc(title),
        badge(status),
    )
}

/// Implemented by per-stage data so the renderer does not need to know each type.
trait RenderStage {
    fn render_rows(&self) -> String;
}

impl RenderStage for crate::model::DnsData {
    fn render_rows(&self) -> String {
        let mut rows = String::new();
        rows.push_str(&kv("resolver", &self.resolver));
        rows.push_str(&list("A", &self.a));
        rows.push_str(&list("AAAA", &self.aaaa));
        rows.push_str(&list("CAA", &self.caa));
        rows.push_str(&kv(
            "min TTL",
            &self
                .min_ttl_secs
                .map(|t| format!("{t} s"))
                .unwrap_or_else(|| "—".to_string()),
        ));
        rows.push_str(&kv("DNSSEC", &self.dnssec));
        if !self.chain.is_empty() {
            let hops: Vec<String> = self
                .chain
                .iter()
                .map(|h| {
                    let ns = if h.ns.is_empty() {
                        "—".to_string()
                    } else {
                        h.ns.join(", ")
                    };
                    match h.ttl_secs {
                        Some(t) => format!("{} → [{ns}] (ttl {t}s)", h.zone),
                        None => format!("{} → [{ns}]", h.zone),
                    }
                })
                .collect();
            rows.push_str(&list("chain", &hops));
        }
        rows.push_str(&list("DS", &self.ds));
        rows.push_str(&list("DNSKEY", &self.dnskey));
        rows.push_str(&list("notes", &self.notes));
        rows
    }
}

impl RenderStage for crate::model::TcpData {
    fn render_rows(&self) -> String {
        let mut rows = String::new();
        rows.push_str(&kv("addr", &self.addr));
        rows.push_str(&kv("connect", &format!("{} ms", self.connect_ms)));
        if self.connect_ms_samples.len() > 1 {
            rows.push_str(&kv("samples", &fmt_samples(&self.connect_ms_samples)));
        }
        rows
    }
}

impl RenderStage for crate::model::TlsData {
    fn render_rows(&self) -> String {
        let mut rows = String::new();
        rows.push_str(&kv("ALPN", self.alpn.as_deref().unwrap_or("—")));
        rows.push_str(&kv("version", &self.protocol_version));
        rows.push_str(&kv("cipher", &self.cipher_suite));
        rows.push_str(&kv("cert subject", &self.cert_subject));
        rows.push_str(&kv("cert issuer", &self.cert_issuer));
        rows.push_str(&kv("cert not after", &self.cert_not_after));
        rows.push_str(&kv("handshake", &format!("{} ms", self.handshake_ms)));
        if self.handshake_ms_samples.len() > 1 {
            rows.push_str(&kv("samples", &fmt_samples(&self.handshake_ms_samples)));
        }
        rows
    }
}

impl RenderStage for crate::model::HttpData {
    fn render_rows(&self) -> String {
        let mut rows = String::new();
        rows.push_str(&kv("version", &self.version));
        rows.push_str(&kv(
            "status",
            &self
                .http_status
                .map(|s| s.to_string())
                .unwrap_or_else(|| "—".to_string()),
        ));
        rows.push_str(&kv("server", self.server.as_deref().unwrap_or("—")));
        rows.push_str(&kv("alt-svc", self.alt_svc.as_deref().unwrap_or("—")));
        if !self.headers.is_empty() {
            let items: Vec<String> = self
                .headers
                .iter()
                .map(|(k, v)| format!("{k}: {v}"))
                .collect();
            rows.push_str(&list("security headers", &items));
        }
        rows.push_str(&kv("body", &format!("{} bytes", self.body_bytes)));
        rows.push_str(&kv("duration", &format!("{} ms", self.duration_ms)));
        rows
    }
}

impl RenderStage for crate::model::Http3Data {
    fn render_rows(&self) -> String {
        let mut rows = String::new();
        rows.push_str(&kv("advertised", yesno(self.advertised)));
        rows.push_str(&kv("negotiated", yesno(self.negotiated)));
        rows.push_str(&kv("ALPN", self.alpn.as_deref().unwrap_or("—")));
        rows.push_str(&kv("fallback used", yesno(self.fallback_used)));
        rows.push_str(&kv("duration", &format!("{} ms", self.duration_ms)));
        rows.push_str(&kv(
            "failure category",
            self.failure_category.as_deref().unwrap_or("—"),
        ));
        rows.push_str(&kv(
            "resolved IP",
            self.resolved_ip.as_deref().unwrap_or("—"),
        ));
        rows.push_str(&kv("note", &self.note));
        rows
    }
}

fn yesno(b: bool) -> &'static str {
    if b { "yes" } else { "no" }
}

fn fmt_samples(samples: &[u64]) -> String {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let n = sorted.len();
    let min = sorted.first().copied().unwrap_or_default();
    let max = sorted.last().copied().unwrap_or_default();
    let median = if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2
    };
    let all: Vec<String> = samples.iter().map(|s| s.to_string()).collect();
    format!(
        "n={n} min={min}ms median={median}ms max={max}ms  [{}]",
        all.join(", ")
    )
}

/// Returns a "silent downgrade" warning banner when h3 was advertised but not connected.
fn downgrade_banner(report: &Report) -> String {
    if let Stage::Ok(h3) = &report.http3 {
        if h3.advertised && !h3.negotiated {
            return "<div class=\"alert\">\
                <strong>Silent HTTP/3 downgrade.</strong> \
                The server advertised <code>h3</code> via <code>Alt-Svc</code>, \
                but the QUIC handshake did not complete, so the connection fell back. \
                This is exactly the kind of claim-not-backed-by-reality WIREGLASS exists to expose.\
                </div>"
                .to_string();
        }
    }
    String::new()
}

/// Render a self-contained HTML report (inline CSS, no external resources).
pub fn render_html(report: &Report) -> String {
    let dns = stage_card("DNS", &report.dns);
    let tcp = stage_card("TCP", &report.tcp);
    let tls = stage_card("TLS", &report.tls);
    let http = stage_card("HTTP", &report.http);
    let http3 = stage_card("HTTP/3 (QUIC)", &report.http3);
    let score_html = report
        .score
        .as_ref()
        .map(crate::score::render_html)
        .unwrap_or_default();
    let meta = format!(
        "host <code>{}</code> · port <code>{}</code> · resolved <code>{}</code> · ran <code>{}</code>",
        esc(&report.host),
        report.port,
        esc(report.resolved_ip.as_deref().unwrap_or("—")),
        esc(&report.ran_at_utc),
    );

    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
<title>wireglass — {target}</title>\n\
<style>{css}</style>\n</head>\n<body>\n\
<header class=\"top\">\n\
  <div class=\"brand\">WIREGLASS</div>\n\
  <div class=\"sub\">See what the connection actually does.</div>\n\
  <h1>{target}</h1>\n\
  <div class=\"meta\">{meta}</div>\n\
</header>\n\
{banner}\n\
<main>\n{dns}\n{tcp}\n{tls}\n{http}\n{http3}\n{score}\n</main>\n\
<footer>Generated by wireglass {version} — local, offline, reproducible. \
No data was sent to any third-party service.</footer>\n\
</body>\n</html>\n",
        target = esc(&report.target),
        css = CSS,
        meta = meta,
        banner = downgrade_banner(report),
        version = esc(&report.version),
        score = score_html,
    )
}

/// Parse a saved report JSON and render it to HTML. This is the reproducible
/// path: an identical JSON always yields identical HTML.
pub fn render_from_json(json: &str) -> Result<String, serde_json::Error> {
    let mut report: Report = serde_json::from_str(json)?;
    if report.score.is_none() {
        report.score = Some(crate::score::compute(&report));
    }
    Ok(render_html(&report))
}

const CSS: &str = r#"
:root{--bg:#0d1117;--card:#161b22;--border:#30363d;--fg:#e6edf3;--muted:#8b949e;
--ok:#238636;--okfg:#3fb950;--fail:#8b2f2f;--failfg:#f85149;--skip:#30363d;--skipfg:#8b949e;}
*{box-sizing:border-box}
body{margin:0;background:var(--bg);color:var(--fg);font:15px/1.5 ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
.top{padding:28px 24px 8px;max-width:960px;margin:0 auto}
.brand{font-weight:700;letter-spacing:.18em;color:var(--okfg)}
.sub{color:var(--muted);font-size:13px;margin-bottom:10px}
h1{font-size:22px;margin:8px 0 6px;word-break:break-all}
.meta{color:var(--muted);font-size:13px}
.meta code{color:var(--fg)}
main{max-width:960px;margin:0 auto;padding:12px 24px 40px}
.card{background:var(--card);border:1px solid var(--border);border-radius:10px;padding:14px 16px;margin:14px 0}
.card.ok{border-left:4px solid var(--okfg)}
.card.failed{border-left:4px solid var(--failfg)}
.card.not_attempted{border-left:4px solid var(--skipfg)}
.card header{display:flex;align-items:center;justify-content:space-between;margin-bottom:8px}
.card h2{font-size:15px;margin:0;letter-spacing:.04em}
.rows{display:grid;gap:2px}
.kv{display:grid;grid-template-columns:140px 1fr;gap:10px;padding:2px 0}
.k{color:var(--muted)}
.v{word-break:break-word}
.v ul{margin:0;padding-left:16px}
.badge{font-size:12px;padding:2px 8px;border-radius:999px}
.badge.ok{background:var(--ok);color:#fff}
.badge.fail{background:var(--fail);color:#fff}
.badge.skip{background:var(--skip);color:var(--skipfg)}
.alert{max-width:960px;margin:14px auto 0;padding:14px 18px;border-radius:10px;
background:#3a1d0e;border:1px solid #d29922;color:#f0d999}
.alert code{color:#ffd479}
footer{max-width:960px;margin:0 auto;padding:0 24px 40px;color:var(--muted);font-size:12px}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DnsData, Http3Data, Report};

    fn sample(report: Report) -> Report {
        report
    }

    fn base() -> Report {
        Report {
            tool: "wireglass".to_string(),
            version: "0.1.0".to_string(),
            target: "https://example.com".to_string(),
            host: "example.com".to_string(),
            port: 443,
            resolved_ip: Some("93.184.216.34".to_string()),
            ran_at_utc: "2026-10-13T00:00:00Z".to_string(),
            dns: Stage::Ok(DnsData::default()),
            tcp: Stage::Failed {
                error: "boom".to_string(),
            },
            tls: Stage::NotAttempted {
                reason: "not run".to_string(),
            },
            http: Stage::NotAttempted {
                reason: "not run".to_string(),
            },
            http3: Stage::Ok(Http3Data::default()),
            dot: Stage::NotAttempted {
                reason: "not run".to_string(),
            },
            score: None,
            ports: Vec::new(),
            udp_measurements: Vec::new(),
        }
    }

    #[test]
    fn escapes_html_in_values() {
        let mut r = base();
        r.host = "<script>alert(1)</script>".to_string();
        r.target = "https://x/<b>".to_string();
        let html = render_html(&r);
        assert!(!html.contains("<script>alert(1)</script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn render_is_deterministic() {
        let r = base();
        assert_eq!(render_html(&r), render_html(&r));
    }

    #[test]
    fn from_json_roundtrip_is_deterministic() {
        let r = base();
        let json = serde_json::to_string(&r).unwrap();
        let a = render_from_json(&json).unwrap();
        let b = render_from_json(&json).unwrap();
        assert_eq!(a, b);
        assert!(a.contains("example.com"));
    }

    #[test]
    fn downgrade_banner_only_when_advertised_not_negotiated() {
        let mut r = base();
        r.http3 = Stage::Ok(Http3Data {
            advertised: true,
            negotiated: false,
            ..Default::default()
        });
        assert!(render_html(&r).contains("Silent HTTP/3 downgrade"));

        r.http3 = Stage::Ok(Http3Data {
            advertised: true,
            negotiated: true,
            ..Default::default()
        });
        assert!(!render_html(&r).contains("Silent HTTP/3 downgrade"));
    }

    #[test]
    fn samples_summary_is_formatted() {
        let s = fmt_samples(&[10, 5, 20, 5]);
        assert!(s.contains("n=4"));
        assert!(s.contains("min=5ms"));
        assert!(s.contains("max=20ms"));
        assert_eq!(sample(base()).host, "example.com");
    }
}
