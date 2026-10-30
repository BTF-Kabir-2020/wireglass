# docs/evidence-plan.md — turning competitor findings into testable criteria (WIREGLASS)

> This document turns `docs/competitors.md` from "analysis" into "runnable tests".
> **Golden rule:** every product claim must have a falsifiable test and recorded evidence. Failed measurements are not hidden; they are marked.
> **Gate:** no new feature is added to the plan until the "Phase 1 pass criteria" is proven with evidence.

---

## 0. How evidence is recorded

For each test, store these under `evidence/` (after the repo is created):

- `evidence/<date>-<host>-<test>.json` — raw JSON output of the tool.
- `evidence/<date>-<host>-<test>.log` — terminal output of the same run (stdout/stderr).
- `evidence/<date>-<host>-notes.md` — run date, tool version, and success/failure interpretation.

Every item below must reference one of these files; a claim without evidence is rejected.

---

## 1. Phase 1 pass criteria — exact tests

Each row: **claim → test method → required evidence → pass condition.**

### G1 — DNS check with real result and real error
- **Claim:** the DNS chain (chain/TTL/DNSSEC/CAA and A/AAAA records) is read with a real resolver, not superficially.
- **Test:** run on a healthy domain (e.g. `example.com`) and on a broken/nonexistent domain (e.g. `nonexistent.invalid`).
- **Evidence:** JSON containing `resolver_used`, `chain[]`, `ttl`, `dnssec`, `caa`, `a`, `aaaa`; and for the nonexistent domain an **explicit error** (not empty output).
- **Pass:** both cases behave correctly; the error is reported with a specific code/message.
- **Differentiation target:** `http-stat-rs`/`httptap` only see "resolved"; we see the chain and DNSSEC/CAA.

### G2 — Independent TCP and TLS measurement
- **Claim:** TCP connect time and TLS handshake time are measured separately and documented.
- **Test:** run at least 5 times on one domain; record dispersion.
- **Evidence:** JSON with `tcp.connect_ms[]`, `tls.handshake_ms[]`, `tls.protocol`, `tls.cipher`, `tls.alpn`, `tls.cert_expiry`.
- **Pass:** both stages have independent values; dispersion is recorded.
- **Differentiation target:** TLS depth similar to `testssl.sh` but in the same single report.

### G3 — Negotiated ALPN (not inferred from headers)
- **Claim:** ALPN is read from the handshake itself (`h2`/`http/1.1`), not from response headers.
- **Test:** a domain that negotiates `h2` and a domain that only offers `http/1.1`.
- **Evidence:** `tls.alpn` equals the actually negotiated value; handshake evidence in the log.
- **Pass:** the ALPN value cannot be swapped by an `Alt-Svc`/`Server` header (negative test: a header claiming something different from the real ALPN must not win).
- **Differentiation target:** separating the source of truth (handshake ≠ header).

### G4 — Real HTTP/2 and HTTP/3 test with advertisement vs connection split
- **Claim:** HTTP/3 is tested via a **real QUIC negotiation**; "support" is not inferred from `Alt-Svc` alone.
- **Test:** three scenarios:
  1. a domain that truly has H3.
  2. a domain that advertises `Alt-Svc: h3` but the QUIC connection does not establish ("silent downgrade").
  3. a domain with no H3 at all.
- **Evidence:** for each scenario the fields `http2.negotiated`, `http3.advertised`, `http3.negotiated`, `http3.fallback_used`, and `error` on failure.
- **Pass:** the three cases are distinct and correctly labeled; "success" is never guessed on failure.
- **Differentiation target:** strongest differentiator — `quicprobe` has the issue but only for H3; here it is combined with the rest of the path.

### G5 — Structured JSON output + reproducible HTML report
- **Claim:** output has a stable, diffable structure; the HTML report is regenerated from the same JSON.
- **Test:** two runs on the same input → compare structure (keys); rebuild HTML from saved JSON.
- **Evidence:** JSON schema (stable keys), and `report.html` built from saved JSON without re-running the scan.
- **Pass:** structural keys are stable (variable values allowed); HTML is rebuilt from JSON only.
- **Note:** "reproducible" was marked weak/rejected in competitors.md; here it is only a quality condition, not a differentiation claim.

### G6 — Runnable tests + English README runnable by others
- **Claim:** another developer can clone and run the project.
- **Test:** follow README commands on a fresh environment (at least simulated); `cargo test` is green.
- **Evidence:** output of `cargo test`, `cargo build --release`, and the README text.
- **Pass:** runs without hidden knowledge; tests pass.
- **Differentiation target:** the "single Rust binary, local, OSS" differentiator only makes sense if it is runnable.

---

## 2. Competitor differentiation claims — falsifiable tests

From competitors.md section 4 (three strong candidates). Each claim must be confirmed or rejected by a test:

| Differentiator | Hypothesis | Falsifiable test | Acceptance result |
|---|---|---|---|
| DNS + TLS + H3 in one report | No single tool gives all three in one output | Run WIREGLASS on one domain → all three parts in **one** JSON | One file, all three parts, no second tool |
| Honesty about silent H3 downgrade | Tools hide H3 failures | Scenario G4.2 (Alt-Svc without QUIC) | Explicit label "advertised, not connected" |
| Local/offline single binary + image report | Competitor output is text or web based | Generate SVG→PNG locally, offline after scan | 1200×630 image, offline |

> **Negative PR test:** for each differentiator, pick one sample domain and record the result so the claim is backed by real evidence, not explanation.

---

## 3. Competitor matrix claims that must be **verified** (not assumed)

competitors.md is based on public documentation; before relying on it, verify these by real execution:

- [ ] `http-stat-rs` really has no DNS chain/DNSSEC/CAA (run it and inspect output).
- [ ] `http-stat-rs` really has no image report.
- [ ] `httptap` really treats DNS shallowly.
- [ ] `quicprobe` really is H3-only.
- [ ] DNSViz really is web-only and does not combine TLS/HTTP.

If any is disproven, update the product plan (private) and competitors.md.

---

## 4. Definition of "done" for Phase 1

Phase 1 is done if and only if:

1. All six G1..G6 are closed with evidence in `evidence/`.
2. The three differentiator tests (section 2) produce showable results on a real domain.
3. No claim without evidence remains in the product plan or README.

After this gate: the final image card and the release plan (distribution section of the private product plan) begin.
