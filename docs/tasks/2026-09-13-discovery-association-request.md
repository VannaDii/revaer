# Logical Discovery Association Request

- Status: In progress
- Date: 2026-09-13
- Operator approval: Not applicable: nonarchitectural task record
- Scope / owner / PR: Shared association create/replace request and focused
  wire tests; integration owner, local and unpublished.
- Approved authority: [ADR 557](../adr/557-root-persistence-contract.md#profiles-and-discovery-associations),
  [ADR 559 G1](../adr/559-media-approval-delta.md#g1-architecture-versus-implementation-detail),
  and [ADR 588](../adr/588-first-release-decision-package.md) retain their exact scopes.
- Requirement ledger: L2/L4/L9 in the
  [completion ledger](../adr/564-media-completion-ledger.md#outside-in-ledger).

## Motivation And Design Notes

The full UI gate still fails at schedule activation because the legacy
path-taking profile cannot establish trusted filesystem identity. The approved
replacement uses a separate logical discovery association, not a relaxed root
check or a different expected HTTP status.

Add the exact eight-field create/replace request in the shared API models.
Construction and map-only JSON decoding require both logical keys, an explicit
relative prefix, the profile UUID and positive PostgreSQL integer version, and
all three boolean modes. Empty prefix explicitly selects the whole attested
root; absent/null prefix is rejected. Unknown fields, duplicate fields,
positional arrays, absolute roots, invalid types and invalid bounds fail with
one value-free error. Debug output omits caller values. Immutable requests
serialize exactly the approved fields without defaults or normalization.

The existing map-only decoder is shared with readiness responses; their wire
shape and checks are unchanged. Requested automatic modes remain intent, not
admission. This does not add routes, conditional-header handling, stored
procedures, root attestation, UI controls or scheduling authority. The 1 MiB
decompressed HTTP bound belongs to the route, not this standalone body decoder.
The coordinated init/root cutover and complete operator workflow remain open.

## Test Coverage Summary

At base `2865b1b9` plus this source delta, `just test-media-root-contract`
passes all 72 tests: 11 association tests and 61 existing input/readiness tests,
zero failures/ignored and 24 unrelated tests filtered by the unchanged recipe.
Vectors exercise all required and duplicate fields, all eight mode combinations,
key/prefix/version boundaries, strict types, explicit whole-root selection,
constructor parity, redacted diagnostics and exact JSON round trips.

The initial focused log is `target/discovery-association-contract-tests.log`.
The first `just lint` run failed on a test's manual empty-string construction;
it is corrected to `String::new()` without changing lint criteria or assertions.
The focused rerun again passes 72 tests, including an additional UUID-array
rejection vector; strict all-target/all-feature model Clippy passes. Formatting,
instruction drift and documentation indexing pass. The private disposable gate
runner now checks final TCP readiness instead of the temporary initialization
socket, with the same 60 attempts and database settings; no child-PGDATA or
unproven ownership workaround is adopted.
Full CI, UI, current-source published Sonar, package and remote-check results
are not established by this focused run. Source-pinned full results and durable
evidence are recorded here as they complete; the record is not a handoff pass.

## Observability And Status Docs

No live log, metric, health, HTTP, SSE or support claim changes. The completion
ledger identifies this as a request representation, not usable discovery.

## Risk, Rollback And Dependencies

Risk is disagreement between the accepted request and its decoder. Independent
wire vectors and existing readiness regressions cover the shared helper move.
Rollback removes the association model/tests and returns that unchanged helper
to readiness; no database or media state changes. No dependency is added.
ADR 589 remains pending and is unrelated to this request representation.

## Stale-Policy Check And Cleanup

Reviewed root, Rust and UI instructions and ADR 557's HTTP and coordinated-cutover
sections. No rule, runtime compatibility contract or quality criterion changes.
Root readiness is unchanged. New routine records follow the current task-record
policy rather than creating an architectural approval event. The conflicted
operator checkout remains untouched. Focused model tests create no media.
