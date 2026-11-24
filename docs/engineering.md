# Engineering design

Architecture and usage are in the [README](../README.md); decisions are in the
[ADRs](adr).

## Boundaries

devpulse is a single binary with four layers. Each depends only on the ones
below it, which is what makes the analysis testable without git.

| Layer | Responsibility | Depends on |
|---|---|---|
| `cli` | Commands, flags, exit codes | config, git, analysis, report |
| `git` | Run `git log`, parse numstat into `Commit` values | model |
| `analysis` | Hotspots, ownership, bus factor, health score | model |
| `report` | table, JSON and Markdown renderers | analysis |

`analysis` is pure: it takes a slice of commits and returns a value. All
I/O is in `git` and `cli`.

## Security design

devpulse reads repositories that may be untrusted (a CI job running on a fork).

| Threat | Mitigation |
|---|---|
| Command injection through paths or refs | `git` invoked with an argument vector, never a shell; user paths passed after `--` |
| Malicious repository config (hooks, `core.fsmonitor`) | Only `git log` is run, which does not execute hooks |
| Data exfiltration | No network access in the binary at all |
| Supply chain | `Cargo.lock` committed, `--locked` builds, Dependabot, checksums published with release assets |

Planned: signed release artifacts with build provenance (M7).

## Performance

A single pass over `git log` output; the
analysis is `O(changes)` with hash maps keyed by path. ripgrep's full history
(2,380 commits) takes about 0.7 s on a laptop, nearly all of it inside `git log`.
For very large monorepos, an incremental cache keyed by commit is planned (M8).

## Observability

A CLI's observability is its output contract: stable JSON schema, documented
exit codes, errors to stderr. `--format json` output is versioned from 1.0 (M10).

## Release and distribution

- **CI:** `fmt`, `clippy -D warnings` and tests on Linux, macOS and Windows; MSRV build.
- **Release:** a `v*` tag builds binaries for Linux (x86_64, aarch64), macOS
  (aarch64, x86_64) and Windows (x86_64), with SHA-256 checksums, and attaches
  them to a GitHub release.
- **Action:** `action.yml` runs the Docker image so CI users need no Rust toolchain.

## Testing strategy

| Layer | What | Where |
|---|---|---|
| Parser | numstat edge cases: renames, binary files, empty commits | `src/git/parser.rs` |
| Analysis | Hotspot ranking, ownership, bus factor, health formula | `src/analysis` |
| End to end | The built binary against temporary git repositories: all formats, `check` exit codes, empty repos | `tests/cli.rs` |
| Planned | Snapshot tests for renderers; fuzzing the parser (M3) | |

## Documentation strategy

README for users (install, usage, the health formula), ADRs for design
choices, `CHANGELOG.md` per release, this document for maintainers.
