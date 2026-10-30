use std::net::IpAddr;
use std::time::Instant;

use anyhow::{Context, Result};
use hickory_resolver::config::{CLOUDFLARE, ResolverConfig};
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use hickory_resolver::proto::dnssec::rdata::DNSSECRData;
use hickory_resolver::proto::rr::{RData, RecordType};
use hickory_resolver::{Resolver, TokioResolver};

use crate::model::{DnsData, DnsHop};

pub struct DnsOutcome {
    pub data: DnsData,
    pub ips: Vec<IpAddr>,
}

/// Prefer OS resolver config. On Android (and some locked-down hosts) reading
/// system DNS via Hickory often fails — fall back to Cloudflare public DNS and
/// label the report honestly so we never pretend it was "system".
fn build_resolver() -> Result<(TokioResolver, String)> {
    match Resolver::builder_tokio().and_then(|b| b.build()) {
        Ok(resolver) => Ok((resolver, "system (hickory TokioResolver)".to_string())),
        Err(sys_err) => {
            let short = sys_err
                .to_string()
                .lines()
                .next()
                .unwrap_or("system DNS unavailable")
                .to_string();
            let resolver = Resolver::builder_with_config(
                ResolverConfig::udp_and_tcp(&CLOUDFLARE),
                TokioRuntimeProvider::default(),
            )
            .build()
            .with_context(|| {
                format!("system DNS unavailable ({short}); Cloudflare 1.1.1.1 fallback also failed")
            })?;
            Ok((
                resolver,
                format!("cloudflare 1.1.1.1 (fallback; system DNS unavailable: {short})"),
            ))
        }
    }
}

pub async fn probe(host: &str) -> Result<DnsOutcome> {
    // If the target is already an IP literal, skip DNS entirely.
    if let Ok(ip) = host.parse::<IpAddr>() {
        let mut data = DnsData {
            resolver: "ip literal (no DNS)".to_string(),
            dnssec: "n/a".to_string(),
            ..Default::default()
        };
        match ip {
            IpAddr::V4(v4) => data.a.push(v4.to_string()),
            IpAddr::V6(v6) => data.aaaa.push(v6.to_string()),
        }
        data.min_ttl_secs = None;
        data.duration_ms = 0;
        return Ok(DnsOutcome {
            data,
            ips: vec![ip],
        });
    }

    let start = Instant::now();
    let fqdn = format!("{host}.");
    let (resolver, resolver_label) = build_resolver()?;

    let mut data = DnsData {
        resolver: resolver_label,
        dnssec: "unknown".to_string(),
        ..Default::default()
    };

    let a = resolver
        .lookup(fqdn.clone(), RecordType::A)
        .await
        .with_context(|| format!("A lookup failed for {fqdn}"))?;

    let mut min_ttl: Option<u32> = None;
    let mut ips: Vec<IpAddr> = Vec::new();

    for rec in a.answers() {
        if let RData::A(ip) = &rec.data {
            let ip: IpAddr = IpAddr::V4(ip.0);
            data.a.push(ip.to_string());
            ips.push(ip);
            min_ttl = Some(min_ttl.map_or(rec.ttl, |m| m.min(rec.ttl)));
        }
    }

    match resolver.lookup(fqdn.clone(), RecordType::AAAA).await {
        Ok(ac) => {
            for rec in ac.answers() {
                if let RData::AAAA(ip) = &rec.data {
                    let ip: IpAddr = IpAddr::V6(ip.0);
                    data.aaaa.push(ip.to_string());
                    ips.push(ip);
                    min_ttl = Some(min_ttl.map_or(rec.ttl, |m| m.min(rec.ttl)));
                }
            }
        }
        Err(e) => data.notes.push(format!("AAAA lookup: {e}")),
    }

    match resolver.lookup(fqdn.clone(), RecordType::CAA).await {
        Ok(ac) => {
            for rec in ac.answers() {
                if let RData::CAA(caa) = &rec.data {
                    data.caa.push(format!(
                        "{} {}",
                        caa.tag,
                        String::from_utf8_lossy(&caa.value)
                    ));
                }
            }
        }
        Err(e) => data.notes.push(format!("CAA lookup: {e}")),
    }

    if data.a.is_empty() && data.aaaa.is_empty() {
        anyhow::bail!("no A/AAAA records returned for {fqdn}");
    }

    // Delegation chain (root -> ... -> zone containing the host).
    data.chain = delegation_chain(&resolver, &fqdn, &mut data.notes).await;
    let apex = data
        .chain
        .last()
        .map(|h| h.zone.clone())
        .unwrap_or_else(|| fqdn.clone());

    // DS lives in the parent zone, so query it for every zone in the chain
    // except the root. Raw records only; signatures are not validated.
    for hop in &data.chain {
        if hop.zone == "." {
            continue;
        }
        data.ds.extend(ds_records(&resolver, &hop.zone).await);
    }

    data.dnskey = dnskey_records(&resolver, &apex).await;

    data.dnssec = if !data.dnskey.is_empty() {
        "signed_unverified".to_string()
    } else if !data.ds.is_empty() {
        "delegation_signed_no_dnskey".to_string()
    } else {
        "unsigned".to_string()
    };

    data.min_ttl_secs = min_ttl;
    data.duration_ms = start.elapsed().as_millis() as u64;
    Ok(DnsOutcome { data, ips })
}

/// Label suffixes of `fqdn`, from the root toward the host.
/// e.g. `www.example.com.` -> `[".", "com.", "example.com.", "www.example.com."]`
fn zone_suffixes(fqdn: &str) -> Vec<String> {
    let trimmed = fqdn.trim_end_matches('.');
    if trimmed.is_empty() {
        return vec![".".to_string()];
    }
    let labels: Vec<&str> = trimmed.split('.').collect();
    let mut out = vec![".".to_string()];
    for i in (0..labels.len()).rev() {
        out.push(format!("{}.", labels[i..].join(".")));
    }
    out
}

async fn delegation_chain(
    resolver: &TokioResolver,
    fqdn: &str,
    notes: &mut Vec<String>,
) -> Vec<DnsHop> {
    let mut chain = Vec::new();
    for zone in zone_suffixes(fqdn) {
        let (ns, ttl) = ns_records(resolver, &zone).await;
        if ns.is_empty() {
            continue;
        }
        let soa = soa_mname(resolver, &zone).await;
        chain.push(DnsHop {
            zone,
            ns,
            soa,
            ttl_secs: ttl,
        });
    }
    if chain.is_empty() {
        notes.push("no NS records observed along the delegation chain".to_string());
    }
    chain
}

async fn ns_records(resolver: &TokioResolver, zone: &str) -> (Vec<String>, Option<u32>) {
    match resolver.lookup(zone.to_string(), RecordType::NS).await {
        Ok(lookup) => {
            let mut ns = Vec::new();
            let mut ttl: Option<u32> = None;
            for rec in lookup.answers() {
                if let RData::NS(n) = &rec.data {
                    ns.push(n.0.to_string());
                    ttl = Some(ttl.map_or(rec.ttl, |m| m.min(rec.ttl)));
                }
            }
            (ns, ttl)
        }
        Err(_) => (Vec::new(), None),
    }
}

async fn soa_mname(resolver: &TokioResolver, zone: &str) -> Option<String> {
    match resolver.lookup(zone.to_string(), RecordType::SOA).await {
        Ok(lookup) => lookup.answers().iter().find_map(|rec| match &rec.data {
            RData::SOA(soa) => Some(soa.mname.to_string()),
            _ => None,
        }),
        Err(_) => None,
    }
}

async fn ds_records(resolver: &TokioResolver, zone: &str) -> Vec<String> {
    match resolver.lookup(zone.to_string(), RecordType::DS).await {
        Ok(lookup) => lookup
            .answers()
            .iter()
            .filter_map(|rec| match &rec.data {
                RData::DNSSEC(DNSSECRData::DS(ds)) => Some(ds.to_string()),
                _ => None,
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

async fn dnskey_records(resolver: &TokioResolver, zone: &str) -> Vec<String> {
    match resolver.lookup(zone.to_string(), RecordType::DNSKEY).await {
        Ok(lookup) => lookup
            .answers()
            .iter()
            .filter_map(|rec| match &rec.data {
                RData::DNSSEC(DNSSECRData::DNSKEY(key)) => Some(key.to_string()),
                _ => None,
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::zone_suffixes;

    #[test]
    fn suffixes_lists_root_to_host() {
        assert_eq!(
            zone_suffixes("www.example.com."),
            vec![".", "com.", "example.com.", "www.example.com."]
        );
    }

    #[test]
    fn suffixes_handles_single_label() {
        assert_eq!(zone_suffixes("com."), vec![".", "com."]);
    }

    #[test]
    fn suffixes_handles_root() {
        assert_eq!(zone_suffixes("."), vec!["."]);
    }
}
