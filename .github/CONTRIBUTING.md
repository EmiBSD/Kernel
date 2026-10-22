# Contributing to EmiBSD

Thanks for your interest. EmiBSD is a one-person experiment: an operating system in Rust,
redesigned from the faithful port of the OpenBSD kernel ([README.md](../README.md)).

## What is welcome

Issues, in English:

- **A bug**: a panic, a hang, a wrong result, a smoke that fails on your machine.
- **A difference from OpenBSD**: OpenBSD's behaviour is the specification, so any place where
  OpenBSD's userland sees something on EmiBSD that it would not see on OpenBSD 8.0 is a bug.
- **A documentation error**: something in `docs/` or the README that is wrong or out of date.
- **An idea or a question** about the design: open an issue and say so.

Use the issue forms; they ask for what is needed to reproduce. Vulnerabilities go through
the private channel in [SECURITY.md](SECURITY.md), never a public issue.

A problem in the port itself (also present in EmiBSD.LZ) can go to
[EmiBSD.LZ's issues](https://github.com/EmiBSD/LandingZone/issues) instead.

## Pull requests

External pull requests are not accepted, for now. Every change here follows a fixed process:
provenance recorded per module in `lineage.toml`, `unsafe` counted against a budget, the
system-call ABI kept byte-identical, numbers in each commit's trailers, and a review before
each merge. That process is still settling. If you have a fix, describe it in an issue (a
diff in the issue is fine) and it will be applied with credit.

## Building and running it yourself

- Setup on macOS: [docs/SETUP.md](../docs/SETUP.md).
- `just build`, then `just run-arm64` to boot in QEMU with the serial console on stdio.
- `just smoke` boots headless and checks the serial output; `just test` runs the host unit
  tests.
- Only arm64 is built and tested until milestone N10; amd64 is frozen until then
  ([docs/ROADMAP.md](../docs/ROADMAP.md)).

## How the work is done

- The process: [docs/PHASE2.md](../docs/PHASE2.md) and the README's
  "Contributing and workflow" section.
- The rules every change follows: [CLAUDE.md](../CLAUDE.md) and [.claude/rules/](../.claude/rules/).
- Where things stand: [docs/STATUS.md](../docs/STATUS.md) and [docs/ROADMAP.md](../docs/ROADMAP.md).

## Conduct

Everyone taking part follows the [Code of Conduct](CODE_OF_CONDUCT.md).
