# ADR-0001: Shell out to git instead of linking libgit2

- Status: accepted
- Date: 2026-11-24

## Context

devpulse needs commit metadata and per-file line counts with rename
detection, `.mailmap` support and `--since` semantics identical to git's.

## Options

1. `git2` (libgit2 bindings): needs a C toolchain, makes static and
   cross-platform release builds harder, and re-implements diff and rename
   heuristics that differ subtly from git's.
2. `gix` (pure Rust): promising, but diff statistics and mailmap handling add
   a large dependency surface for one command.
3. **Run `git log --numstat` with a custom record format and parse it.**

## Decision

Option 3. Output is produced with ASCII record (`0x1E`) and unit (`0x1F`)
separators, which cannot occur in commit metadata, and `core.quotepath=off`
so paths are not escaped. The parser is a small, fully tested module.

## Consequences

- Results match what developers see in `git log`, including renames and
  `.mailmap`.
- Requires `git` on `PATH`; the Docker image and GitHub Action ship it.
- Spawning one process per run is negligible next to reading history.
