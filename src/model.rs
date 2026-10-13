use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum Stage<T> {
    #[serde(rename = "ok")]
    Ok(T),
    #[serde(rename = "failed")]
    Failed { error: String },
    #[serde(rename = "not_attempted")]
    NotAttempted { reason: String },
}

#[allow(dead_code)]
impl<T> Stage<T> {
    pub fn status(&self) -> &'static str {
        match self {
            Stage::Ok(_) => "ok",
            Stage::Failed { .. } => "failed",
            Stage::NotAttempted { .. } => "not_attempted",
        }
    }

    pub fn as_ok(&self) -> Option<&T> {
        match self {
            Stage::Ok(v) => Some(v),
            _ => None,
        }
    }

    pub fn error(&self) -> Option<&str> {
        match self {
            Stage::Failed { error } => Some(error),
            _ => None,
        }
    }
}

/// One step of the DNS delegation chain, from a zone toward the host.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DnsHop {
    pub zone: String,
    #[serde(default)]
    pub ns: Vec<String>,
    #[serde(default)]
    pub soa: Option<String>,
    #[serde(default)]
    pub ttl_secs: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DnsData {
    pub resolver: String,
    #[serde(default)]
    pub a: Vec<String>,
    #[serde(default)]
    pub aaaa: Vec<String>,
    #[serde(default)]
    pub caa: Vec<String>,
    /// Delegation chain (root -> ... -> host zone). Roots recorded as NS/SOA hops.
    #[serde(default)]
    pub chain: Vec<DnsHop>,
    /// DS record(s) published by the parent for this zone (raw, unvalidated).
    #[serde(default)]
    pub ds: Vec<String>,
    /// DNSKEY record(s) published at the zone apex (raw, unvalidated).
    #[serde(default)]
    pub dnskey: Vec<String>,
    pub min_ttl_secs: Option<u32>,
    /// Honest status string, e.g. "not_implemented", "unsigned", "signed_unverified".
    pub dnssec: String,
    pub duration_ms: u64,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TcpData {
    pub addr: String,
    pub connect_ms: u64,
    /// All independent connect samples when `--repeat` > 1 (empty otherwise).
    #[serde(default)]
    pub connect_ms_samples: Vec<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TlsData {
    pub alpn: Option<String>,
    pub protocol_version: String,
    pub cipher_suite: String,
    pub cert_subject: String,
    pub cert_issuer: String,
    pub cert_not_after: String,
    pub handshake_ms: u64,
    /// All independent handshake samples when `--repeat` > 1 (empty otherwise).
    #[serde(default)]
    pub handshake_ms_samples: Vec<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HttpData {
    pub version: String,
    pub http_status: Option<u16>,
    pub server: Option<String>,
    pub alt_svc: Option<String>,
    pub body_bytes: usize,
    pub duration_ms: u64,
    /// Sanitized response headers kept for transparent scoring (no secrets).
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Http3Data {
    pub advertised: bool,
    pub negotiated: bool,
    pub alpn: Option<String>,
    pub fallback_used: bool,
    pub duration_ms: u64,
    pub note: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScoreItem {
    pub category: String,
    pub value: u8,
    pub max: u8,
    pub why: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Score {
    pub total: u8,
    pub grade: String,
    pub items: Vec<ScoreItem>,
    pub roast: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub tool: String,
    pub version: String,
    pub target: String,
    pub host: String,
    pub port: u16,
    pub resolved_ip: Option<String>,
    pub ran_at_utc: String,
    pub dns: Stage<DnsData>,
    pub tcp: Stage<TcpData>,
    pub tls: Stage<TlsData>,
    pub http: Stage<HttpData>,
    pub http3: Stage<Http3Data>,
    #[serde(default)]
    pub score: Option<Score>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_tags_are_stable() {
        let not_run: Stage<()> = Stage::NotAttempted {
            reason: "not run".to_string(),
        };
        let v = serde_json::to_value(&not_run).unwrap();
        assert_eq!(v["status"], "not_attempted");

        let failed: Stage<()> = Stage::Failed {
            error: "boom".to_string(),
        };
        let v = serde_json::to_value(&failed).unwrap();
        assert_eq!(v["status"], "failed");
        assert_eq!(v["error"], "boom");
    }

    #[test]
    fn ok_variant_flattens_without_tag_collision() {
        let ok: Stage<HttpData> = Stage::Ok(HttpData {
            version: "2".to_string(),
            http_status: Some(200),
            ..Default::default()
        });
        let v = serde_json::to_value(&ok).unwrap();
        assert_eq!(v["status"], "ok");
        assert_eq!(v["version"], "2");
        assert_eq!(v["http_status"], 200);
    }

    #[test]
    fn report_roundtrips_through_json() {
        let report = Report {
            tool: "wireglass".to_string(),
            version: "0.1.0".to_string(),
            target: "https://example.com".to_string(),
            host: "example.com".to_string(),
            port: 443,
            resolved_ip: Some("93.184.216.34".to_string()),
            ran_at_utc: "2026-10-13T00:00:00Z".to_string(),
            dns: Stage::Ok(DnsData {
                a: vec!["93.184.216.34".to_string()],
                dnssec: "signed_unverified".to_string(),
                ..Default::default()
            }),
            tcp: Stage::Failed {
                error: "nope".to_string(),
            },
            tls: Stage::NotAttempted {
                reason: "not run".to_string(),
            },
            http: Stage::Ok(HttpData::default()),
            http3: Stage::Ok(Http3Data::default()),
            score: None,
        };

        let json = serde_json::to_string(&report).unwrap();
        let back: Report = serde_json::from_str(&json).unwrap();
        assert_eq!(back.host, "example.com");
        assert_eq!(back.dns.as_ok().unwrap().a, vec!["93.184.216.34"]);
        assert!(back.tcp.error().is_some());
        assert_eq!(back.http3.status(), "ok");
    }
}
