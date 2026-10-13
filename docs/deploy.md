# Deploy / publish WIREGLASS

## Local gates (must pass before publishing)
```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
cargo run --example quic_selfcheck
git status --short
```
Optional public server:
```sh
cargo build --release --features server
WIREGLASS_PORT=8080 ./target/release/wireglassd
```

## Docker
```sh
docker build -t wireglass .
docker run --rm wireglass https://example.com
```

## GitHub repository (from a clean checkout)
```sh
gh auth login
# Check that BTF-Kabir-2020/wireglass is still unused:
gh repo view BTF-Kabir-2020/wireglass 2>/dev/null && echo "NAME TAKEN"
git init
git checkout -b main
git add .
git commit -m "Initial WIREGLASS release"
gh repo create BTF-Kabir-2020/wireglass --public --source . --remote origin --push
```
Do not force-add ignored private files:
- `PLAN.md`
- `LAUNCH.md`
- `evidence/`

## After creation
1. Upload `assets/github-social-preview.png` under Settings → Social preview.
2. Set repository About and topics from `LAUNCH.md`.
3. Enable branch protection: run CI on pull requests and pushes.
4. Cut first release tag `v0.1.0`; Release Drafter will fill notes.
5. The CI workflow already runs fmt, clippy, build and tests.

## Cross-checks
- Search GitHub repositories for `wireglass` before launch: other projects exist using the same name, but no Rust network connection-path CLI. Full path `BTF-Kabir-2020/wireglass` should be checked immediately before creation.
- Check crates.io for `wireglass` package availability before `cargo publish`; this CLI is optional and should not block the GitHub public release.
