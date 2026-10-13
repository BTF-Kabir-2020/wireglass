use crate::model::{Report, Score, ScoreItem};

fn grade_for(total: u8) -> &'static str {
    match total {
        0..=39 => "disaster",
        40..=69 => "tolerable",
        70..=89 => "clean",
        _ => "near-perfect",
    }
}

fn roast_for(report: &Report, h3_fail: bool) -> String {
    // Deterministic by host: the same host always gets the same one-liner.
    let mut h: u64 = 5381;
    for b in report.host.as_bytes() {
        h = h.wrapping_mul(33).wrapping_add(u64::from(*b));
    }
    let base = match h % 5 {
        0 => "TLS handshake was fast, but the report is the real story.",
        1 => "A pretty timeline is still not proof. This one has proof.",
        2 => "One real measurement beats a hundred green badges.",
        3 => "The network spoke; the headers only lied. We listened to the network.",
        _ => "Nothing was inferred. Nothing was hidden. That is the whole product.",
    };
    let mut roast = String::new();
    if h3_fail {
        roast.push_str(
            "Alt-Svc promised h3 and the handshake declined: silent downgrade documented. ",
        );
    }
    if report
        .dns
        .as_ok()
        .map(|d| !d.ds.is_empty() && d.dnskey.is_empty())
        .unwrap_or(false)
    {
        roast.push_str("DNSSEC advertises a delegation signature, but no DNSKEY came back. ");
    }
    if roast.is_empty() {
        roast.push_str("No headline failure to mock; still, measurement is not reassurance. ");
    }
    roast.push_str(base);
    roast
}

pub fn compute(report: &Report) -> Score {
    let mut items = Vec::new();

    // Transfer security (25)
    let tls_score = match report.tls.as_ok() {
        Some(t) => {
            let mut s: u8 = 10;
            if t.protocol_version == "TLSv1_3" {
                s = s.saturating_add(10);
            }
            if t.alpn.as_deref() == Some("h2") {
                s = s.saturating_add(5);
            }
            s.min(25)
        }
        None => 0,
    };
    items.push(ScoreItem {
        category: "transfer security".to_string(),
        value: tls_score,
        max: 25,
        why: match report.tls.status() {
            "ok" => "TLS handshake completed; TLS 1.3 and negotiated h2 both boost the score."
                .to_string(),
            "failed" => "TLS handshake failed, so no security evidence exists.".to_string(),
            _ => "TLS was not attempted.".to_string(),
        },
    });

    // Modern transfer (20)
    let http_score = match report.http.as_ok() {
        Some(h) => {
            let mut s: u8 = 0;
            if h.version == "2" {
                s = s.saturating_add(10);
            }
            if report
                .http3
                .as_ok()
                .map(|h3| h3.negotiated)
                .unwrap_or(false)
            {
                s = s.saturating_add(10);
            }
            s.min(20)
        }
        None => 0,
    };
    items.push(ScoreItem {
        category: "modern transfer".to_string(),
        value: http_score,
        max: 20,
        why: "HTTP/2 over TCP is the floor; real HTTP/3 negotiation doubles it.".to_string(),
    });

    // DNS (20)
    let dns_score = match report.dns.as_ok() {
        Some(d) => match d.dnssec.as_str() {
            "signed_unverified" => 20,
            "delegation_signed_no_dnskey" => 10,
            "unsigned" => 0,
            _ => 5,
        },
        None => 0,
    };
    items.push(ScoreItem {
        category: "dns".to_string(),
        value: dns_score,
        max: 20,
        why: "Chain, DS and DNSKEY collected; signatures are not yet verified.".to_string(),
    });

    // Headers security (20): scored only from headers actually measured.
    let headers_score = match report.http.as_ok() {
        Some(h) => {
            let essential = [
                "strict-transport-security",
                "content-security-policy",
                "x-frame-options",
                "x-content-type-options",
            ];
            let light = [
                "referrer-policy",
                "permissions-policy",
                "cross-origin-opener-policy",
                "cross-origin-resource-policy",
                "cross-origin-embedder-policy",
            ];
            let mut s: u8 = 0;
            for name in essential {
                if h.headers.contains_key(name) {
                    s += 4;
                }
            }
            s += (light.iter().filter(|n| h.headers.contains_key(**n)).count() as u8).min(4);
            s.min(20)
        }
        None => 0,
    };
    items.push(ScoreItem {
        category: "headers / privacy".to_string(),
        value: headers_score,
        max: 20,
        why: "Security headers are now measured from the real response; missing ones cost points."
            .to_string(),
    });
    // Content/SEO are not measured yet.
    items.push(ScoreItem {
        category: "content / seo base".to_string(),
        value: 0,
        max: 15,
        why: "robots.txt, sitemap, canonical and metadata are not measured yet (Phase 1)."
            .to_string(),
    });

    let mut total: u8 = 0;
    for item in &items {
        total = total.saturating_add(item.value);
    }
    let total = total.min(100);
    let h3_fail = report
        .http3
        .as_ok()
        .map(|h3| h3.advertised && !h3.negotiated)
        .unwrap_or(false);
    let roast = roast_for(report, h3_fail);

    Score {
        total,
        grade: grade_for(total).to_string(),
        items,
        roast,
    }
}

/// Render a tiny, self-contained score block for the HTML report.
pub fn render_html(score: &Score) -> String {
    let mut items = String::new();
    for item in &score.items {
        items.push_str(&format!(
            "<div class=\"kv\"><span class=\"k\">{} ({}/{})</span><span class=\"v\">{}</span></div>",
            item.category, item.value, item.max, item.why
        ));
    }
    format!(
        "<section class=\"card ok\">\
           <header><h2>Score: {} / 100 — {}</h2></header>\
           <div class=\"rows\">{}\
             <div class=\"kv\"><span class=\"k\">roast</span><span class=\"v\">{}</span></div>\
           </div>\
         </section>",
        score.total, score.grade, items, score.roast
    )
}
