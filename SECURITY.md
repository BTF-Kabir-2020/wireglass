# Security Policy

## Supported versions

WIREGLASS is an early skeleton. Security fixes are applied to the latest `main`
only. There are no long-term support branches yet.

| Version | Supported |
|---|---|
| 0.1.x (latest `main`) | Yes |
| older | No |

## Reporting a vulnerability

**Please do not open a public issue for security problems.**

Report privately using GitHub's **"Report a vulnerability"** button on the
repository's *Security* tab (private vulnerability reporting). If that is not
available, contact a maintainer directly through their public GitHub profile.

Please include:

- what you found and where (file, module, or command),
- steps to reproduce, ideally a minimal input,
- the impact you believe it has,
- any suggested fix, if you have one.

We will acknowledge your report as quickly as we can and keep you updated on the
fix and disclosure timeline. Please give us reasonable time to release a fix
before disclosing publicly.

## Scope and threat model

WIREGLASS is designed to run **locally**, on your own machine, against hosts you
choose. It talks directly to DNS/TCP/TLS/QUIC endpoints and prints a report.

Relevant areas:

- **Secrets in reports.** Reports must never contain tokens, cookies, or private
  data. If you find a code path that emits sensitive response data, that is a
  security bug.
- **The future public/hosted service (not built yet).** A hosted version that
  accepts arbitrary user URLs introduces SSRF risk (internal IPs, cloud
  metadata, redirects, DNS rebinding). That version must ship with SSRF guards,
  per-redirect re-validation, time/size limits, and rate limiting **before** it
  is exposed. This repository does not yet provide that service.

Out of scope: findings that require running a malicious *local* configuration,
or that only affect a dependency without a reachable path in this tool.
