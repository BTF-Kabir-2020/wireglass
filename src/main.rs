use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;

use wireglass::{Report, UdpMeasurement, run};

#[derive(Parser)]
#[command(
    name = "wireglass",
    version,
    about = "See what the connection actually does."
)]
struct Cli {
    url: Option<String>,
    #[arg(long, default_value_t = 10)]
    timeout: u64,
    #[arg(long, default_value_t = false)]
    compact: bool,
    #[arg(long, value_name = "FILE")]
    html: Option<PathBuf>,
    #[arg(long, value_name = "FILE")]
    render: Option<PathBuf>,
    #[arg(long, value_name = "FILE")]
    out: Option<PathBuf>,
    #[arg(long, default_value_t = 1)]
    repeat: u32,
    #[arg(long = "stun-target", value_name = "HOST:PORT", action = clap::ArgAction::Append)]
    stun_targets: Vec<String>,
    #[arg(long = "quic-target", value_name = "HOST:PORT", action = clap::ArgAction::Append)]
    quic_targets: Vec<String>,
    #[arg(long, value_name = "FILE")]
    card: Option<PathBuf>,
    outdir: Option<PathBuf>,
}

fn parse_endpoint(value: &str) -> anyhow::Result<(&str, u16)> {
    let (host, port) = if let Some(bracketed) = value.strip_prefix('[') {
        let (host, rest) = bracketed
            .split_once(']')
            .ok_or_else(|| anyhow::anyhow!("invalid endpoint {value:?}: missing ]"))?;
        (
            host,
            rest.strip_prefix(':')
                .ok_or_else(|| anyhow::anyhow!("invalid endpoint {value:?}: missing port"))?,
        )
    } else {
        let (host, port) = value
            .rsplit_once(':')
            .ok_or_else(|| anyhow::anyhow!("endpoint must be HOST:PORT, got {value:?}"))?;
        if host.contains(':') {
            anyhow::bail!("IPv6 endpoints must be bracketed: {value:?}");
        }
        (host, port)
    };
    let port = port
        .parse::<u16>()
        .map_err(|_| anyhow::anyhow!("invalid port in endpoint {value:?}"))?;
    if host.is_empty() || port == 0 {
        anyhow::bail!("endpoint must have a host and nonzero port: {value:?}");
    }
    Ok((host, port))
}

#[derive(Clone, Copy)]
enum UdpProbe {
    Stun,
    Quic,
}

async fn probe_targets(
    targets: &[(&str, u16)],
    probe: UdpProbe,
    timeout: Duration,
    repeat: u32,
) -> Vec<UdpMeasurement> {
    let mut measurements = Vec::with_capacity(targets.len());
    for &(host, port) in targets {
        let measurement = match probe {
            UdpProbe::Stun => wireglass::udp::probe_stun(host, port, timeout, repeat).await,
            UdpProbe::Quic => wireglass::udp::probe_quic(host, port, timeout, repeat).await,
        };
        measurements.push(measurement);
    }
    measurements
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let cli = Cli::parse();

    if let Some(path) = &cli.render {
        let json = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("reading {}: {e}", path.display()))?;
        let html = wireglass::report::render_from_json(&json)?;
        match &cli.out {
            Some(out) => std::fs::write(out, html)?,
            None => print!("{html}"),
        }
        return Ok(());
    }

    if cli.url.is_none() && cli.stun_targets.is_empty() && cli.quic_targets.is_empty() {
        anyhow::bail!(
            "missing <URL> (or provide --stun-target/--quic-target, or use --render <report.json>)"
        );
    }
    let stun_targets: Vec<_> = cli
        .stun_targets
        .iter()
        .map(|target| parse_endpoint(target))
        .collect::<anyhow::Result<_>>()?;
    let quic_targets: Vec<_> = cli
        .quic_targets
        .iter()
        .map(|target| parse_endpoint(target))
        .collect::<anyhow::Result<_>>()?;

    if cli.url.is_none() {
        let timeout = Duration::from_secs(cli.timeout);
        let mut measurements =
            probe_targets(&stun_targets, UdpProbe::Stun, timeout, cli.repeat).await;
        measurements
            .extend(probe_targets(&quic_targets, UdpProbe::Quic, timeout, cli.repeat).await);
        let report = wireglass::UdpReport {
            tool: "wireglass".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            ran_at_utc: time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
            udp_measurements: measurements,
        };
        let json = if cli.compact {
            serde_json::to_string(&report)?
        } else {
            serde_json::to_string_pretty(&report)?
        };
        println!("{json}");
        return Ok(());
    }

    let mut report: Report = run(cli.url.as_deref().unwrap(), cli.timeout, cli.repeat).await;
    let timeout = Duration::from_secs(cli.timeout);
    report
        .udp_measurements
        .extend(probe_targets(&stun_targets, UdpProbe::Stun, timeout, cli.repeat).await);
    report
        .udp_measurements
        .extend(probe_targets(&quic_targets, UdpProbe::Quic, timeout, cli.repeat).await);
    let json = if cli.compact {
        serde_json::to_string(&report)?
    } else {
        serde_json::to_string_pretty(&report)?
    };
    println!("{json}");

    if let Some(path) = &cli.html {
        std::fs::write(path, wireglass::report::render_html(&report))?;
        eprintln!("wrote HTML report to {}", path.display());
    }
    if let Some(path) = &cli.card {
        std::fs::write(path, wireglass::card::render_svg(&report))?;
        eprintln!("wrote card to {}", path.display());
    }
    if let Some(dir) = &cli.outdir {
        std::fs::create_dir_all(dir)?;
        let json = serde_json::to_string_pretty(&report)?;
        std::fs::write(dir.join("report.json"), json)?;
        std::fs::write(
            dir.join("report.html"),
            wireglass::report::render_html(&report),
        )?;
        std::fs::write(dir.join("card.svg"), wireglass::card::render_svg(&report))?;
        eprintln!("wrote artifacts to {}", dir.display());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{UdpProbe, parse_endpoint, probe_targets};
    use clap::Parser;
    use std::time::Duration;

    #[test]
    fn parses_hostname_and_port() {
        assert_eq!(
            parse_endpoint("stun.example.test:3478").unwrap(),
            ("stun.example.test", 3478)
        );
    }

    #[test]
    fn parses_bracketed_ipv6_and_port() {
        assert_eq!(
            parse_endpoint("[2001:db8::1]:3478").unwrap(),
            ("2001:db8::1", 3478)
        );
    }

    #[test]
    fn accepts_repeated_stun_and_quic_options_together() {
        let parsed = super::Cli::try_parse_from([
            "wireglass",
            "--stun-target",
            "stun.example.test:3478",
            "--quic-target",
            "example.test:8443",
        ]);
        assert!(parsed.is_ok());
    }

    #[test]
    fn rejects_unbracketed_ipv6_and_invalid_ports() {
        assert!(parse_endpoint("2001:db8::1:3478").is_err());
        assert!(parse_endpoint("host:0").is_err());
        assert!(parse_endpoint("host:nope").is_err());
    }

    #[tokio::test]
    async fn multi_target_probe_preserves_ports_and_resolution_failures() {
        let targets = [("bad host", 3478), ("bad host", 3479)];
        let results = probe_targets(&targets, UdpProbe::Stun, Duration::from_millis(50), 2).await;
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].port, 3478);
        assert_eq!(results[1].port, 3479);
        for result in results {
            assert_eq!(result.status, "resolution_error");
            assert_eq!(result.attempts, 0);
        }
    }
}
