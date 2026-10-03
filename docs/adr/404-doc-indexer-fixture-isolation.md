# Doc Indexer Fixture Isolation

- Status: Accepted
- Date: 2026-08-04

## Motivation

The doc-indexer test helper derived temporary directory names from wall-clock
nanoseconds and then reused an existing path through `create_dir_all`. Parallel
or repeated same-process tests could therefore share fixture state.

## Design Notes

- Locate the repository through the canonical `AGENTS.md` file.
- Combine the process id with a process-local atomic sequence.
- Reserve each fixture root atomically with `std::fs::create_dir`.
- Retry only `AlreadyExists`; propagate every other filesystem error.

## Test Coverage Summary

- `cargo test -p revaer-doc-indexer`
- `just lint`
- `just ci`
- `just ui-e2e`

## Observability Updates

No production logs, metrics, or events change because this is test-only fixture
isolation.

## Risk And Rollback Plan

The sequence is process-local, while the process id separates concurrent test
processes. The atomic directory reservation remains the authoritative collision
check. Roll back this ADR and helper change together if the test harness adopts
a repository-standard temporary-directory abstraction.

## Dependency Rationale

No dependencies were added; the implementation uses `std` only.

## Stale-Policy Check

Reviewed `AGENTS.md` and `.github/instructions/rust.instructions.md`. The stale
`AGENT.md` repository-root sentinel was corrected; no other drift was found.
