# Media planned job compliance report slice 4/8

- Status: Accepted
- Date: 2026-06-01
- Context:
  - The core compliance scorer produces normalized status, severity, and violation rows.
  - Runtime planned jobs previously carried operations and workspace estimates only, so preflight callers could not persist or display compliance findings without recomputing the diff.
- Decision:
  - Add the core compliance report to `PlannedJob`.
  - Populate the report during inspected-source planning from the same graph diff used to generate operations.
  - Keep manually constructed test plans explicit by using compliant status reports.
- Consequences:
  - Positive outcomes:
    - Preflight, API, persistence, and UI layers can consume one planned-job payload for operations, workspace sizing, and compliance findings.
    - Compliance facts stay tied to the deterministic diff that produced the plan.
  - Risks or trade-offs:
    - `PlannedJob` construction now requires a compliance report in tests and future helper code.
- Follow-up:
  - Persist compliance violations and expose them through job preview/detail APIs.

## Task Record

- Motivation:
  - Continue slice 4/8 by carrying compliance reports out of runtime planning instead of leaving them as core-only helpers.
- Design notes:
  - Reused `score_diff` inside `plan_job_from_source_graph` before operation generation.
  - Kept the field structured rather than serializing a JSON blob, preserving the normalized persistence path required by the plan.
- Test coverage summary:
  - Added a failing runtime test assertion that a video codec mismatch planned job exposes `Status::NonCompliant` and at least one violation.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-media-runtime -- --test-threads=1`.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo clippy -p revaer-media-runtime --all-targets --all-features -- -D warnings -W clippy::cargo -W clippy::nursery -A clippy::multiple_crate_versions -A clippy::redundant_pub_crate`.
- Observability updates:
  - None. This creates structured data for later persistence, audit, and event emission.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: downstream test fixtures must now include compliance reports when constructing planned jobs. Roll back by removing the `PlannedJob` field, runtime scorer call, test assertion, and this ADR.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
