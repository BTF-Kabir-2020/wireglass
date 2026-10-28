# AGENTS.md

Guidance for AI coding agents (and humans) working in this repository. Keep it
short and factual.

## What this project is

WIREGLASS is a local-only Rust CLI that walks the real connection path to a URL —
DNS → TCP → TLS (ALPN) → HTTP/2 → HTTP/3 — and reports each step as a directly
measured, reproducible result. See `README.md` and `docs/evidence-plan.md`.

## Commands

```sh
cargo fmt --all                             # format
cargo clippy --all-targets -- -D warnings   # lint (warnings are errors in CI)
cargo test                                  # unit tests (fast, offline)
cargo run -- https://example.com            # real scan (network required)
cargo run --example quic_selfcheck          # loopback QUIC proof (no network needed)
```

Always run `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, and
`cargo test` before considering a task done.

## Hard rules

- **Measure, never infer.** Every value in the report must come from a real
  measurement. If something fails, label it as failed with the error — do not
  hide it or guess a value.
- **Advertised is not working.** A protocol advertised via `Alt-Svc` must never
  be reported as connected unless a real handshake succeeded.
- **No secrets in output.** Never emit tokens, cookies, or private data.
- **Local-only.** Do not add calls to third-party services.
- **SSRF guards are mandatory** for any future service that accepts user URLs.

## Conventions

- Rust edition 2024, stable toolchain (1.85+).
- One connection stage per module in `src/` (`dns.rs`, `net.rs`, `tls.rs`,
  `http.rs`, `h3.rs`); report types live in `model.rs`.
- Keep dependencies minimal; justify any new crate in the change description.
- Add unit tests next to the code. Network tests belong in `examples/` and must
  never be required by `cargo test`.
- Windows builds use a larger main-thread stack (see `.cargo/config.toml`); do
  not remove it, hickory's resolver can overflow the default 1 MB stack.
- Write comments sparingly and only where the reason is non-obvious.

## Do not commit

`PLAN.md`, `LAUNCH.md`, and `evidence/` are private/local and git-ignored (see `.gitignore`).
Do not force-add them.
