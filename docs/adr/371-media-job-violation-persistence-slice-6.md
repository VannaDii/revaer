# Media job violation persistence slice 6

- Status: Accepted
- Date: 2026-06-01
- Context:
  - `MEDIA_TRANSCODING.md` requires normalized job violation persistence rather than JSONB.
  - Runtime planning now produces structured compliance reports, but the data layer had no table or stored procedures for violation rows.
- Decision:
  - Add `media_job_violation` with one row per job violation and deterministic per-job ordering.
  - Add stored procedures for appending/upserting and listing violation rows by job public id.
  - Add Rust data-layer wrappers that call only those stored procedures.
- Consequences:
  - Positive outcomes:
    - Compliance violations can be persisted in normalized rows tied to media jobs.
    - Future API and UI job detail work can list violation rows without recomputing the plan.
  - Risks or trade-offs:
    - The current row shape stores normalized kind, severity, and optional stream id only; richer expected/actual values may need a follow-up normalized table or columns.
- Follow-up:
  - Wire runtime compliance reports into job execution persistence and expose violations through API job detail surfaces.

## Task Record

- Motivation:
  - Continue slice 6 by adding the normalized violation storage required before job compliance can be persisted end to end.
- Design notes:
  - Used a separate table keyed by `media_job_id` and `violation_index` to avoid JSONB and support deterministic ordering.
  - Kept runtime access through stored procedures only.
  - Normalized severity with a database check constraint for `low`, `medium`, and `high`.
- Test coverage summary:
  - Added failing schema expectations for `media_job_violation`, `media_job_violation_append_v1`, and `media_job_violation_list_v1`.
  - Added failing data-layer tests for append/list and closed-pool error propagation.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-data media_ -- --test-threads=1`.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo clippy -p revaer-data --all-targets --all-features -- -D warnings -W clippy::cargo -W clippy::nursery -A clippy::multiple_crate_versions -A clippy::redundant_pub_crate`.
- Observability updates:
  - None. This is persistence structure for future job/audit surfaces.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: a later richer compliance model may require additional columns. Roll back by reverting migration `0130`, data wrappers/tests, schema expectations, and this ADR before shipping migrations to production.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
