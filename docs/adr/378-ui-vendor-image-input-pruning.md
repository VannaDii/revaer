# UI vendor image input pruning

- Status: Recorded
- Date: 2026-07-31
- Operator approval: Not applicable: nonarchitectural task record.
- Context:
  - The PR Sonar scan reported invalid UTF-8 warnings for committed raster image
    assets.
  - Revaer policy treats scanner warnings as required fixes and forbids hiding
    committed assets through Sonar exclusions without explicit operator consent.
  - Nexus `public/images` was not an input to the runtime asset-sync pipeline.
- Decision:
  - Remove unused tracked files under
    `crates/revaer-ui/ui_vendor/nexus-html@3.1.0/public/images`.
  - Keep runtime asset inputs required by `asset_sync` unchanged.
  - Do not relax Sonar analysis scope, suffix handling, source encoding, or test
    classification.
- Consequences:
  - The committed vendor surface and invalid-encoding warning sources shrink
    without changing runtime assets.
  - Future work that needs one of the removed Nexus samples must import that
    specific asset through a reviewed ingestion change.
- Follow-up:
  - Resolve required runtime raster inputs through accepted ADR 379 rather than
    inferring a format or scanner exception from this deletion.

## Task Record

- Motivation:
  - Delete unused scanner inputs instead of weakening analysis criteria.
- Design notes:
  - The removed directory was not referenced by runtime code, asset-sync code,
    Justfile recipes, or scoped instructions.
- Test coverage summary:
  - Asset synchronization, asset-sync unit tests, policy, instruction drift, and
    diff checks cover the recorded cleanup.
- Observability updates:
  - None; runtime behavior does not change.
- Status-doc validation:
  - The ADR index and documentation summary reference this record.
- Risk & rollback plan:
  - Restore only a specifically required asset if a future validated pipeline
    proves it necessary.
- Dependency rationale:
  - No dependency was added.
- Stale-policy check:
  - Reviewed root, UI, Sonar, and DevOps instructions. No contradiction was
    retained.
