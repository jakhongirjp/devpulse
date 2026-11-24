# ADR-0002: Pure analysis core with a single I/O boundary

- Status: accepted
- Date: 2026-11-24

## Decision

Reading history is behind the `git::History` trait. `analysis` and `report`
are pure functions over `model` types: no file system, no clock (the current
time is passed in), no environment.

## Consequences

- Metrics are tested with hand-built commits in microseconds, including edge
  cases (empty history, ties, renames) that are awkward to create in a repo.
- A future cache or a hosted mode can implement `History` without touching
  the metrics.
- End-to-end tests still run the real binary against temporary repositories
  so the boundary itself is covered.
