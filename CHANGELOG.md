# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Protocol-aware UDP diagnostics.** `--stun-target HOST:PORT` sends a real
  STUN Binding request and accepts only a correlated Binding Success Response
  with the expected magic cookie, transaction ID, declared message length, and
  bounded attributes. `--repeat` and `--timeout` apply per measurement.
- **Multi-target / multi-port probing.** Repeat `--stun-target` and/or
  `--quic-target`; each target carries its own port. QUIC probes reuse the
  existing HTTP/3 handshake instead of a second implementation.
- **Structured UDP JSON.** UDP-only runs emit `udp_measurements`; URL runs keep
  every existing field and append the same array. Categories are `success`,
  `timeout`, `protocol_error`, and `resolution_error`. A timeout is never
  reported as proof that UDP is blocked.
- Deterministic local-UDP unit tests for valid Binding responses (with and
  without attributes), mismatched transaction IDs, wrong cookie/type, malformed
  and truncated lengths, arbitrary payloads, timeout, and DNS failure.

### Changed

- The HTTP/3 probe accepts a destination port, so URL scans now use the URL's
  actual port instead of always testing 443.
- `http3` JSON may include `failure_category` and `resolved_ip`; both fields are
  optional, so older reports still deserialize.

## [0.1.0] - 2026-10-13

### Added

- Initial Phase 0 skeleton: DNS (A/AAAA/CAA), TCP connect timing, TLS handshake
  with negotiated ALPN/cipher/version and certificate info, HTTP/2 request, and
  a real QUIC/HTTP-3 handshake probe that distinguishes *advertised* (`Alt-Svc`)
  from *actually connected*.
- **DNS delegation chain**: the report now walks the NS/SOA chain from the root
  toward the host zone and records each hop (zone, NS set, SOA MNAME, TTL).
- **DNS DNSSEC raw records**: `DS` and `DNSKEY` records are collected, with an
  honest `dnssec` status of `unsigned`, `signed_unverified`, or
  `delegation_signed_no_dnskey`. Signatures are **not** cryptographically
  validated.
- Structured JSON report with per-stage `ok` / `failed` / `not_attempted`
  status, plus explicit errors instead of guesses.
- `--repeat N` to record TCP/TLS timing dispersion.
- Self-contained HTML report (`--html FILE`) and offline re-render from a saved
  JSON report (`--render FILE [--out FILE]`).
- Loopback QUIC self-check example (`cargo run --example quic_selfcheck`).
- Deterministic SVG report card (`--card FILE`, 1200×630) alongside the existing cover/PNG assets.
- Interpretive score (`score/total`, `grade`, per-category values and reasons, deterministic `roast`) in the JSON and HTML report.
- HTML rendering of the score card.

### Changed

- The DNS stage reports real data instead of `dnssec: "not_implemented"`.

[0.1.0]: https://github.com/BTF-Kabir-2020/wireglass/releases/tag/v0.1.0
[Unreleased]: https://github.com/BTF-Kabir-2020/wireglass/compare/v0.1.0...HEAD
