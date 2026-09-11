# Root Readiness Response Contract

- Status: In progress
- Date: 2026-09-11
- Operator approval: Not applicable: nonarchitectural task record
- Scope / owner / PR: Shared API-model readiness representation and independent
  contract tests; local unpublished continuation from `bb302c03`.
- Approved authority: [ADR 557](../adr/557-root-persistence-contract.md#root-administration)
  R1-R4 and [ADR 559](../adr/559-media-approval-delta.md)'s narrow G1. The
  [approval register](../media-approval-register.md) retains every pending hold.
- Requirement ledger: [L2 and L9](../adr/564-media-completion-ledger.md#outside-in-ledger).

## Motivation And Design Notes

The accepted outside-in sequence starts with the HTTP contract. The existing
root input helpers establish grammar but cannot represent the path-free
readiness result. Add that result under `media_root_contract` without installing
a route, changing a provider, or replacing legacy database/root authority.

Construction and deserialization enforce format version 1, the exact closed
source and attestation states/reasons, and a positive decimal generation bounded
to PostgreSQL `bigint`. A missing source is distinct from a ready zero-slot
catalog. Each result contains exactly five ordered kinds, with
`destructive <= binding <= attested <= 256`. Unavailable generations have zero
counts. Optional fields serialize as absent, never as invented empty values.
Unknown top-level and per-kind fields are rejected; no paths, logical keys, slot
ids, filesystem identities, owner values or digests exist in the response.

The public model is immutable after validation. Deserialization errors translate
to one bounded diagnostic without caller data. This validates reported values,
not the truth of filesystem probes or the consistency of future SQL reads.
Authentication, real routes, root administration, stored procedures, final init,
UI and packaged evidence remain separate work. No accepted or held runtime
behavior is changed.

## Test Coverage Summary

Independent JSON vectors cover exact state mappings, wire shape, omission,
generation precision, count bounds and ordering, malformed/duplicate/unknown
fields, and privacy. Constructor tests separately exercise the typed entry
point and its failure boundary. Run through existing
`just test-media-root-contract`, then the unchanged full gates. Results and exact
tested revision will be recorded after execution; prior passes do not certify
this change.

- Initial focused Linux run: exit 101, 57 passed and one failed. Independent
  tests demonstrated that Serde's derived struct decoder accepted positional
  JSON arrays. Retained the exact dirty implementation patch over `9edbfb6e`
  and the failure log; no negative expectation was removed.
- Fixed the representation using a map-only Serde visitor, preserving the
  derived field, type and duplicate checks without a second JSON parser.
  Added raw-JSON duplicate-field regressions and standalone row checks.
- Focused rerun: exit 0, 60 passed, zero failed or ignored, 24 unrelated tests
  filtered by the existing recipe. This includes 38 readiness tests and the
  22 existing root-input tests. Linux arm64 validation image identity is
  `sha256:64bc48fceaca573e4fdd6173ebb7a9113216d51b0ecd4c2fe2320b463dad4992`;
  this Ubuntu build environment is not a supported production image.
- `just fmt`, `just instruction-drift` and whitespace validation pass.
  Documentation links passed at the earlier draft with 1,249 checked and zero
  errors. `just docs-build` exited 0 but retained the 12,418,905-byte search-index
  warning. Final documentation checks and full CI/UI validation remain pending.
- The attempted `just --command sonar analyze secrets` file list did not run:
  sandboxed keychain/state access failed, and the permission reviewer rejected
  the broader source upload on retry. Requested operator consent for this exact
  deliverable's changed source/tests/docs under `VannaDii_Revaer`; the previous
  `final_sql.rb` consent is not reused. No clean Sonar or published coverage
  result is claimed.

## Observability And Status Docs

No metrics, logs, health routes, HTTP errors or SSE behavior changes. The
bounded model error contains no supplied values. This record and the completion
ledger retain the distinction between a transport contract and a usable root
workflow; no release or support claim is advanced.

## Risk, Rollback And Dependencies

The main risk is a representation that disagrees with the accepted contract;
independent wire vectors and constructor tests target that boundary. Revert the
isolated model/tests if needed; no database, media or deployment needs rollback.

Use existing standard-library and Serde dependencies. Add only the workspace's
already locked `serde_json` as a dev dependency of `revaer-api-models`, to test
the actual JSON representation independently. No new package, version or
production dependency is introduced. C1, D4, D5, E1, S2 and other held choices
remain unapproved.

## Stale-Policy Check And Cleanup

Reviewed `AGENTS.md`, Rust, data, devops and Sonar instructions. The frozen SQL
and coordinated-cutover constraints rule out changing current root persistence
in this slice. Added a Rust contract reminder; no criteria, exception, numeric
choice, workflow or recipe was changed. Routine documentation follows ADR 587;
no new architectural decision is asserted.

The primary checkout's existing workflow changes and documentation conflicts
are preserved. Parent and independent test work use separate temporary
worktrees; cleanup and final validation evidence will be recorded at closeout.
