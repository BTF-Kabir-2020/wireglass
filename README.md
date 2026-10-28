# WIREGLASS

[![CI](https://github.com/BTF-Kabir-2020/wireglass/actions/workflows/ci.yml/badge.svg)](https://github.com/BTF-Kabir-2020/wireglass/actions/workflows/ci.yml)
[![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

Project page: https://ghourchiani.ir/projects/wireglass

> **See what the connection actually does.**
> A local-only Rust CLI that walks the real path of a connection — DNS → TCP → TLS (ALPN) → HTTP/2 → HTTP/3 — and reports each step as a directly measured, reproducible result.

![WIREGLASS — See what the connection actually does](assets/og-cover.png)

Status: **Phase 1 — scoring + interpretive roast on top of the Phase 0 measurement engine.** Report, score, and honest failures are all in one JSON. See `docs/evidence-plan.md`.

## What it actually measures

One command, one URL, one report:

- **DNS** — A / AAAA records with real TTLs, CAA records, the NS/SOA **delegation chain** (root → … → host zone), raw **DS / DNSKEY** records, and explicit errors (via `hickory-resolver`).
- **TCP** — real connect timing to a chosen address.
- **TLS** — real handshake timing, negotiated protocol version, cipher suite, **negotiated ALPN**, and certificate subject/issuer/expiry (via `rustls` + `x509-parser`).
- **HTTP/2** — a real request over the negotiated ALPN, capturing status, `server`, and `Alt-Svc`.
- **HTTP/3** — a **real QUIC handshake** with ALPN `h3`. The report distinguishes *advertised* (`Alt-Svc`) from *actually connected*. A protocol being advertised is never reported as working.

Every failed measurement is reported with its error. Nothing is inferred or guessed. That honesty is the point.

## Why it is different

Per `docs/competitors.md`, "a timeline" and "a reproducible report" alone are **not** unique — `http-stat-rs` and `httptap` already cover that. WIREGLASS bets on the combination:

1. Deep DNS + TLS + HTTP/2 + HTTP/3 in one reproducible output.
2. Honest reporting of the **silent HTTP/3 downgrade** (advertised vs. actually connected).
3. Local, offline, single-binary Rust — no third-party service sees your data.

## Run it

```sh
cargo run -- https://example.com
cargo run -- https://example.com --compact
cargo run -- https://example.com --timeout 15
cargo run -- https://example.com --card card.svg --outdir evidence/example-com
```

### Docker

```sh
docker build -t wireglass .
docker run --rm wireglass https://example.com
```

Output is JSON on stdout (pretty by default).

### Verify the QUIC/HTTP-3 client path

Network may block outbound UDP/443, which makes real HTTP/3 hosts time out. To prove the client itself works, run the loopback self-check:

```sh
cargo run --example quic_selfcheck
```

Expected:

```
[client] QUIC handshake OK, negotiated alpn=Some("h3")
[server] accepted QUIC connection, alpn=Some("h3")
```

## Example report (trimmed)

```json
{
  "host": "example.com",
  "resolved_ip": "104.20.23.154",
  "dns":   { "status": "ok", "a": ["104.20.23.154"], "aaaa": ["2606:4700:10::6814:179a"], "min_ttl_secs": 159,
             "chain": [".", "com.", "example.com."], "dnssec": "signed_unverified" },
  "tcp":   { "status": "ok", "connect_ms": 2 },
  "tls":   { "status": "ok", "alpn": "h2", "protocol_version": "TLSv1_3", "cert_not_after": "2026-12-25T22:56:35Z" },
  "http":  { "status": "ok", "version": "2", "http_status": 200, "alt_svc": "h3=\":443\"; ma=86400" },
  "http3": { "status": "ok", "advertised": true, "negotiated": false, "fallback_used": true,
             "note": "Alt-Svc advertised h3 but QUIC handshake failed: ..." }
}
```

![Terminal preview of a wireglass run](assets/terminal-preview.png)

## Honest limitations (current skeleton)

- **DNSSEC validation:** the delegation chain and raw `DS` / `DNSKEY` records are reported, but signatures are **not cryptographically validated**. `dnssec` is one of `unsigned`, `signed_unverified`, or `delegation_signed_no_dnskey` (the last means a parent `DS` existed but the resolver returned no `DNSKEY`). Full chain-of-trust verification is not implemented.
- **HTTP/3 request/response:** only the QUIC handshake is implemented. The report says so explicitly instead of pretending.
- **IPv4 preferred** for the connection step; IPv6-only hosts still resolve but are not connected to.

## Project layout

```
src/
  main.rs    # CLI (clap) + orchestration
  model.rs   # report types (serde)
  dns.rs     # DNS collectors
  net.rs     # TCP connect
  tls.rs     # TLS handshake, ALPN, certificate info
  http.rs    # HTTP/2 (h2) and HTTP/1.1 request
  h3.rs      # QUIC/HTTP-3 handshake probe
examples/
  quic_selfcheck.rs   # loopback proof that the QUIC client works
docs/
  competitors.md      # competitor research
  evidence-plan.md    # competitors turned into acceptance tests
```

Raw run logs/JSON are kept in a local `evidence/` folder (git-ignored) — they are
reproducible on demand rather than committed.

## Tests

```sh
cargo test
cargo run --example quic_selfcheck
```

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) and the
[Code of Conduct](CODE_OF_CONDUCT.md). Before opening a pull request, run
`cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`.

- Questions and usage help: [SUPPORT.md](SUPPORT.md).
- Security reports: [SECURITY.md](SECURITY.md) (privately, not as a public issue).
- How decisions are made: [GOVERNANCE.md](GOVERNANCE.md).

## Status

Early skeleton. The acceptance gate in `docs/evidence-plan.md` must pass before
any new feature, image card, or public release work begins.

## License

Licensed under either of:

- MIT license ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
