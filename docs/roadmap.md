# Roadmap

Open work for each milestone is tracked as
[GitHub issues](https://github.com/jakhongirjp/devpulse/issues).

| # | Milestone | Outcome | Status |
|---|---|---|---|
| M1 | Core analysis | Hotspots, ownership, bus factor, health score, three output formats | Done (v0.1) |
| M2 | CI integration | `check` command, `.devpulse.toml`, GitHub Action, release binaries | Done (v0.1) |
| M3 | Accuracy | Consistent date semantics, author aliases in config (on top of `.mailmap`, already honoured), parser fuzzing, renderer snapshots | Planned |
| M4 | Temporal coupling | Files that change together, with confidence scores | Planned |
| M5 | Team view | Map authors to teams through CODEOWNERS; per-directory ownership | Planned |
| M6 | Trends | Compare two periods and show the direction of each metric | Planned |
| M7 | Distribution | Signed artifacts, Homebrew tap, crates.io release | Planned |
| M8 | Scale | Incremental cache for monorepos with hundreds of thousands of commits | Planned |
| M9 | Reports | Self-contained HTML report with charts | Planned |
| M10 | 1.0 | Stable JSON schema and CLI flags | Planned |

## Release plan

| Version | Contents |
|---|---|
| v0.1 | M1 and M2 |
| v0.2 | M3 |
| v0.3 | M4 and M5 |
| v0.4 | M6 and M7 |
| v0.5 | M8 and M9 |
| v1.0 | M10 |
