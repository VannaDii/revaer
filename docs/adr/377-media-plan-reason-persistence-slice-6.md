# Media plan reason persistence slice 6

- Status: Accepted
- Date: 2026-06-01
- Context:
  - The media plan requires normalized selected-plan and rejected-plan reasons, but persistence only stored job operations and violations.
  - Planner explanations must remain queryable without storing JSON blobs.
- Decision:
  - Add `media_job_plan_reason` with indexed, normalized reason rows tied to a media job.
  - Add append/list stored procedures and Rust data helpers.
  - Store candidate index, selected flag, stable reason code, bounded text, and creation timestamp.
- Consequences:
  - Positive outcomes:
    - Plan explanations can be persisted and surfaced through runtime/app/API layers in follow-up slices.
    - The schema keeps explanations normalized and ordered without JSONB.
  - Risks or trade-offs:
    - Reason text is stored as bounded text by convention for now; future artifact references can handle larger diagnostics.
- Follow-up:
  - Carry plan reasons through runtime, app, API, and UI job detail surfaces.
  - Persist verification checks and compact audit facts in adjacent schema slices.

## Task Record

- Motivation:
  - Continue slice 6 by adding normalized persistence for planner explanation rows.
- Design notes:
  - Used stored procedures for runtime access and kept raw SQL confined to migrations/tests.
  - Upserted by `(media_job_id, reason_index)` so planners can re-run deterministic reason emission.
- Test coverage summary:
  - Added failing data-layer assertions for appending and listing media job plan reasons, then implemented the table/procs/helpers.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-data create_and_list_media_job -- --test-threads=1`.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-data media_job_queries_surface_query_errors_without_database -- --test-threads=1`.
- Observability updates:
  - None. This is persistence plumbing for later diagnostic surfaces.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: reason taxonomy may need more columns as planner selection expands. Roll back by dropping the migration/helper additions and this ADR before release.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
