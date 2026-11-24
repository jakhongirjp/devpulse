# CTO review checklist

Where the project stands. Unchecked items have an issue.

## Design
- [x] Pure analysis layer, tested without git
- [x] Health formula documented and unit tested
- [x] Decisions recorded as ADRs
- [ ] JSON output schema versioned (M10)

## Correctness
- [x] Rename and binary-file handling tested
- [x] End-to-end tests against real temporary repositories
- [ ] `--since` filter and the displayed date range use the same date (M3)
- [x] Author identities merged via `.mailmap` (`%aN`)
- [ ] Author aliases in `.devpulse.toml` for identities `.mailmap` misses (M3)
- [ ] Parser fuzzing (M3)

## Security
- [x] No shell, no network; paths passed after `--`
- [x] Locked dependency builds, Dependabot
- [x] Release checksums
- [ ] Signed artifacts with provenance (M7)

## Delivery
- [x] CI on Linux, macOS and Windows, plus MSRV
- [x] Tag-driven release for five targets
- [x] GitHub Action for zero-install use
- [ ] Published to crates.io and Homebrew (M7)
