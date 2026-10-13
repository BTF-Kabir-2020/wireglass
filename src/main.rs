use std::path::PathBuf;

use clap::Parser;

use wireglass::{Report, run};

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
    #[arg(long, value_name = "FILE")]
    card: Option<PathBuf>,
    outdir: Option<PathBuf>,
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

    let Some(url) = cli.url.clone() else {
        anyhow::bail!("missing <URL> (or use --render <report.json>)");
    };

    let report: Report = run(&url, cli.timeout, cli.repeat).await;
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
