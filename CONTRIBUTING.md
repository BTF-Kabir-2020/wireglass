# Contributing to WIREGLASS

Thanks for taking the time to contribute. This project is a **local-only network
behavior lab**: it walks the real connection path (DNS → TCP → TLS/ALPN →
HTTP/2 → HTTP/3) and reports each step as a directly measured result.

The one rule that matters most: **every reported value must come from a real
measurement.** Nothing is inferred or guessed, and a failed step is labeled as
failed — never hidden.

## Development setup

- **Rust:** stable toolchain (edition 2024, so Rust **1.85+**). Install via
  [rustup](https://rustup.rs).
- No system services are required. Everything runs locally.

```sh
git clone <your-fork-url>
cd wireglass
cargo build
cargo test
```

## Common commands

```sh
cargo fmt --all                          # format
cargo clippy --all-targets -- -D warnings # lint (warnings are errors in CI)
cargo test                               # unit tests (fast, offline)
cargo run -- https://example.com         # run a real scan
cargo run --example quic_selfcheck       # prove the QUIC client works on loopback
```

CI runs `fmt`, `clippy`, and `test` on Linux/macOS/Windows. Please run the same
commands before opening a pull request.

## What to work on

- Issues labeled `good first issue` are scoped to be approachable.
- Changes that touch a measurement stage (DNS, TCP, TLS, HTTP, HTTP/3) should
  say **what is measured** and **what happens when it fails**.
- The acceptance gate lives in `docs/evidence-plan.md`. Do not weaken the
  honesty contract to make output look better.

## Coding conventions

- Match the existing style; run `cargo fmt` before committing.
- Keep dependencies minimal — justify any new crate in the pull request.
- One stage per module under `src/` (`dns.rs`, `net.rs`, `tls.rs`, `http.rs`,
  `h3.rs`); report types live in `model.rs`.
- Add unit tests next to the code. Network-dependent checks belong in
  `examples/` or must be opt-in and never required for `cargo test`.
- Keep comments rare and useful; let names do the work.
- Never log or emit secrets, tokens, or private data.

## Commit and pull request

- Keep pull requests focused on one change.
- Write a short, plain commit message that says what changed and why.
- Fill out the pull request template — small screenshots or JSON snippets of a
  real run are very welcome.

## Reporting bugs and security issues

- Bugs and feature requests: open an issue (templates are provided).
- Security issues: **do not** open a public issue — see [SECURITY.md](SECURITY.md).

## License

By contributing, you agree that your contributions are dual-licensed under the
[MIT](LICENSE-MIT) and [Apache-2.0](LICENSE-APACHE) licenses, as described in
[this section of the Apache license](LICENSE-APACHE).
