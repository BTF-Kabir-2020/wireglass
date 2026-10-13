# Governance

This is a small, early-stage project. This document describes how decisions are
made so contributors know what to expect.

## Roles

- **Maintainer** — has commit access, reviews and merges pull requests, cuts
  releases, and has the final say on disputes. Currently the project founder.
- **Contributor** — anyone who opens an issue, a pull request, or a discussion.

## How decisions are made

- **Lazy consensus.** Most changes go ahead if no one objects within a
  reasonable review window.
- **Discussion first for big changes.** New stages, dependencies, public API, or
  anything that changes the honesty contract (see below) should start as an
  issue before code is written.
- **Maintainer tie-break.** If consensus is not reached, the maintainer decides
  and records the reasoning in the relevant issue or pull request.

## Non-negotiable project principles

Any change must preserve these, or it will be rejected:

1. **Real measurements only.** Reported values come from an actual measurement;
   a failed step is labeled as failed, never guessed.
2. **Advertised ≠ working.** A protocol being advertised (for example `Alt-Svc`)
   is never reported as a working connection.
3. **No secrets in reports.** Output must not leak tokens, cookies, or private
   data.
4. **Local-first.** The core tool runs on the user's machine and sends data to no
   third-party service.

## Becoming a maintainer

Contributors who consistently make high-quality contributions and help others
may be invited to become maintainers by the current maintainer.

## Changing this document

Open a pull request. Governance changes require the maintainer's approval.
