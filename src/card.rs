use crate::model::{Report, Stage};

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

fn short(s: &str, n: usize) -> String {
    let mut t = s.chars().take(n).collect::<String>();
    if s.chars().count() > n {
        t.push('…');
    }
    t
}

pub fn render_svg(report: &Report) -> String {
    let title = esc(short(&report.host, 34).as_str());
    let score = report.score.as_ref();
    let score_text = match score {
        Some(s) => format!("{}/100 — {}", s.total, esc(&s.grade)),
        None => "—".to_string(),
    };
    let roast = esc(&short(
        report
            .score
            .as_ref()
            .map(|s| s.roast.as_str())
            .unwrap_or("No score computed."),
        88,
    ));

    let dns_extra = match report.dns.as_ok() {
        Some(d) => format!(
            "chain {} · DS {} · DNSKEY {} · {}",
            d.chain.len(),
            d.ds.len(),
            d.dnskey.len(),
            esc(&d.dnssec)
        ),
        None => match report.dns.error() {
            Some(e) => esc(short(e, 48).as_str()),
            None => "not attempted".to_string(),
        },
    };
    let tcp_extra = match report.tcp.as_ok() {
        Some(t) => format!("connect {} ms", t.connect_ms),
        None => match report.tcp.error() {
            Some(e) => esc(short(e, 48).as_str()),
            None => "not attempted".to_string(),
        },
    };
    let tls_extra = match report.tls.as_ok() {
        Some(t) => format!(
            "ALPN {} · {} · {} ms",
            t.alpn.as_deref().unwrap_or("—"),
            esc(&t.protocol_version),
            t.handshake_ms
        ),
        None => match report.tls.error() {
            Some(e) => esc(short(e, 48).as_str()),
            None => "not attempted".to_string(),
        },
    };
    let http_extra = match report.http.as_ok() {
        Some(h) => format!(
            "v{} · status {} · alt-svc {}",
            esc(&h.version),
            h.http_status
                .map(|s| s.to_string())
                .unwrap_or_else(|| "—".to_string()),
            esc(short(h.alt_svc.as_deref().unwrap_or("—"), 36).as_str())
        ),
        None => match report.http.error() {
            Some(e) => esc(short(e, 48).as_str()),
            None => "not attempted".to_string(),
        },
    };
    let h3_extra = match report.http3.as_ok() {
        Some(h) => format!(
            "advertised {} · negotiated {}",
            yesno(h.advertised),
            yesno(h.negotiated)
        ),
        None => match report.http3.error() {
            Some(e) => esc(short(e, 48).as_str()),
            None => "not attempted".to_string(),
        },
    };

    // Build stage rows with running y (static formula).
    let dns_row = stage_row(196, "DNS", "#79c0ff", &dns_extra, status_color(&report.dns));
    let tcp_row = stage_row(244, "TCP", "#79c0ff", &tcp_extra, status_color(&report.tcp));
    let tls_row = stage_row(292, "TLS", "#79c0ff", &tls_extra, status_color(&report.tls));
    let http_row = stage_row(
        340,
        "HTTP",
        "#79c0ff",
        &http_extra,
        status_color(&report.http),
    );
    let h3_row = stage_row(
        388,
        "HTTP/3",
        "#79c0ff",
        &h3_extra,
        status_color(&report.http3),
    );

    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1200\" height=\"630\" \
viewBox=\"0 0 1200 630\" role=\"img\" aria-label=\"WIREGLASS report card\">\
<style>{css}</style>\
<rect width=\"1200\" height=\"630\" fill=\"#0b0f14\"/>\
<text x=\"40\" y=\"60\" class=\"brand\">WIREGLASS</text>\
<text x=\"40\" y=\"90\" class=\"sub\">See what the connection actually does.</text>\
<text x=\"40\" y=\"140\" class=\"host\">{title}</text>\
<text x=\"40\" y=\"170\" class=\"meta\">score: {score}</text>\
{dns}{tcp}{tls}{http}{h3}\
<text x=\"40\" y=\"450\" class=\"roast\">{roast}</text>\
<text x=\"40\" y=\"590\" class=\"foot\">local, offline, reproducible — no data sent to third parties</text>\
</svg>",
        css = CSS,
        title = title,
        score = score_text,
        dns = dns_row,
        tcp = tcp_row,
        tls = tls_row,
        http = http_row,
        h3 = h3_row,
        roast = roast
    )
}

fn status_color<T>(stage: &Stage<T>) -> &'static str {
    match stage.status() {
        "ok" => "#3fb950",
        "failed" => "#f85149",
        _ => "#8b949e",
    }
}

fn stage_row(y: u32, name: &str, _color: &str, extra: &str, color: &str) -> String {
    format!(
        "<text x=\"40\" y=\"{y}\" font-size=\"20\" fill=\"{color}\" font-weight=\"700\">{name} {badge}</text>\
         <text x=\"180\" y=\"{y}\" font-size=\"18\" fill=\"#c9d1d9\">{extra}</text>",
        y = y,
        badge = "",
        color = color,
        extra = extra
    )
    .replace("{badge}", "")
}

fn yesno(b: bool) -> &'static str {
    if b { "yes" } else { "no" }
}

const CSS: &str = "text.brand{font:700 30px system-ui,sans-serif;fill:#3fb950;letter-spacing:.18em}\
text.sub{font:400 15px system-ui,sans-serif;fill:#8b949e}\
text.host{font:700 24px system-ui,sans-serif;fill:#e6edf3}\
text.meta{font:400 18px system-ui,sans-serif;fill:#79c0ff}\
text.roast{font:500 18px system-ui,sans-serif;fill:#c9d1d9}\
text.foot{font:400 13px system-ui,sans-serif;fill:#6e7681}";
