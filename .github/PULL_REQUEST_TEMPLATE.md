# Pull request

## What does this change?

<!-- A short description. Link related issues with "Closes #123". -->

## How was it measured or verified?

<!--
This project reports real measurements only. Show the evidence for your change:
the commands you ran and, where relevant, trimmed JSON/output from a real run.
-->

```
# e.g.
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

## Checklist

- [ ] `cargo fmt --all` was run.
- [ ] `cargo clippy --all-targets -- -D warnings` is clean.
- [ ] `cargo test` passes.
- [ ] Every reported value still comes from a real measurement (nothing inferred or guessed).
- [ ] A failure path is labeled as failed, not hidden.
- [ ] No secrets, tokens, cookies, or private data are emitted.
- [ ] New dependencies are minimal and justified above.
- [ ] Unit tests were added/updated next to the code (if behavior changed).
