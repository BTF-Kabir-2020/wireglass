# docs/competitors.md — Competitor research and market gaps (WIREGLASS)

> **Phase 0 gate.** This document fixes what WIREGLASS shows better and what it does not, before any code is written.
> Research date: 2026-10. Data from official docs, public repositories, and tool websites. Numbers are evidence, not permanent commitments.

---

## 0. Executive summary (honest result)

The differentiator claimed in the product plan — "one unified timeline of connection stages + a reproducible report" — is already occupied by two active tools, including a Rust one:

- **`http-stat-rs`** (Rust, MIT, pure, single binary, zero dependencies): HTTP/1.1 + HTTP/2 + HTTP/3, DNS/TCP/TLS/Server/Transfer timeline, `Server-Timing` parsing, TCP and TLS diagnostics. (~21 stars, active, created 2025-04)
- **`httptap`** (Python, Apache-2.0): waterfall with fade, redirect chain with per-step timing, JSON output, SLO threshold checks. (~809 stars)

**Result:** if WIREGLASS is only "another timeline", it is a duplicate of existing tools and fails our own start condition.

The real gap is not "having a timeline", but three things:

1. **Depth combined into one report:** timeline tools only see DNS as "resolved" (they do not follow the DNS chain/DNSSEC/CAA/TTL); TLS and DNS tools are separate. None gives "the real path from DNS to final ALPN" in one diffable output.
2. **Honest HTTP/3:** tools that advertise H3 do not always prove the QUIC handshake; "support" is often inferred from `Alt-Svc`.
3. **Local/offline single binary + image report:** most tools are text/web oriented; reproducible SVG/PNG cards are rare.

---

## 1. Competitor matrix (short form)

| Tool | Type | Strength | Weakness vs WIREGLASS |
|---|---|---|---|
| http-stat-rs | Rust CLI | H1/H2/H3 timeline, TCP/TLS diag | No DNS chain/DNSSEC/CAA/TTL; no image report |
| httptap | Python CLI | Waterfall, redirects, JSON, SLO | DNS is shallow; no TLS depth; Python runtime |
| DNSViz | Web | DNS/DNSSEC visualization | Web-only; no TLS/HTTP in one report |
| testssl.sh | Bash | Deep TLS | No DNS/HTTP path; heavy |
| quicprobe | CLI | H3 handshake | H3 only; no full path |

---

## 2. Candidate differentiators (to be tested, not assumed)

1. **DNS depth in one report:** chain, TTL, DNSSEC, CAA, DS/DNSKEY (raw, honestly labeled).
2. **Honest H3:** distinguish advertised (`Alt-Svc`) from actually connected (QUIC handshake).
3. **Single local binary + reproducible image report:** offline SVG/PNG card.

---

## 3. Negative PR test

For each differentiator, pick a real domain and record the result so the claim is backed by evidence, not explanation.

---

## 4. Claims that must be verified (not assumed)

- [ ] `http-stat-rs` really has no DNS chain/DNSSEC/CAA (run it and inspect output).
- [ ] `http-stat-rs` really has no image report.
- [ ] `httptap` really treats DNS shallowly.
- [ ] `quicprobe` really is H3-only.
- [ ] DNSViz really is web-only and does not combine TLS/HTTP.

If any is disproven, update the product plan and this file.

---

## 5. Definition of "done" for Phase 0

Phase 0 is done if and only if:

1. Each of G1..G6 (see evidence-plan.md) is closed with evidence in `evidence/`.
2. The three differentiators (section 2) produce showable results on a real domain.
3. No claim without evidence remains in the product plan or README.
