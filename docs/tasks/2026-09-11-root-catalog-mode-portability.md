# Root Catalog Mode Portability

- Status: In progress
- Date: 2026-09-11
- Operator approval: Not applicable: nonarchitectural task record
- Scope / owner / PR: Integrating agent; root-catalog mode conversion and its
  existing evidence regression; unpublished child of governance `d5662941`.
- Approved authority: ADR 550 source trust contract and ADR 559 narrow G1.
  No new architecture, platform support, failure classification or trust rule.
- Requirement ledger: [L2](../adr/564-media-completion-ledger.md#outside-in-ledger).

## Motivation And Design Notes

Canonical Linux `just ci` on the governance tree reached Clippy, then failed
three `useless_conversion` diagnostics in `root_catalog/local_file.rs` at the
file evidence and directory/file trust checks. The native `st_mode` is `u32`
on Linux but narrower on macOS. Blindly removing widening would break the
development build; filtering through permission-only flags would lose evidence.

A single private generic lossless conversion normalizes the native value at
the existing three boundaries. `Into<u32>` accepts both native widths without
casts, truncation, new I/O, extra failure states, cfg-dependent behavior or lint
suppression. Strengthen the existing real-file load test to compare the entire
recorded mode with `MetadataExt::mode()`, retaining the permission assertion.
The trust mask remains `0o022`; ownership, symlink, race and platform gates are
unchanged. No dependency or public API changes.

## Test Coverage Summary

Evidence directory: `/private/tmp/revaer-delivery-governance-evidence-20260911`.
Pre-fix `ci-volume.log` records exit 101 from the three Linux lint errors.
The earlier `ci.log` stopped at disposable bind-mount database ownership; an
owned Docker volume resolved startup without changing the gate. Initial
governance policy checks pass with a private GPG home.

Focused Linux/macOS root-catalog, strict lint and full CI/UI results are pending.
No media conversion, package support or published Sonar result follows from
this correction. No held C1, D4, D5, E1, S2 or other decision is released.

## Observability And Status Docs

No runtime telemetry or diagnostic change. The full mode evidence is unchanged.
Link this record from the task index, book navigation and completion ledger;
leave the specification's scope and architectural approvals intact.

## Risk, Rollback And Dependencies

Risk is accidentally changing the evidence or trust predicate across platforms;
the full-mode comparison and existing rejection/race tests are required. Revert
only this bounded correction to restore the original conversion call sites and
Linux lint failure. No dependency, gate, migration or package input changes.

## Stale-Policy Check And Cleanup

Reviewed AGENTS.md, Rust/FFI scoped instructions, source/evidence types and
existing root-catalog tests. Added the exact cross-platform preservation note
to Rust guidance. No architecture/policy contradiction is reinterpreted as
approval. Work is isolated from the conflicted primary checkout; validation
resources and test media must be removed before handoff.
