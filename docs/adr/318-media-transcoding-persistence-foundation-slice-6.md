# Media transcoding persistence foundation slice 6

- Status: Accepted
- Date: 2026-05-23
- Context:
  - `MEDIA_TRANSCODING.md` slice 6 requires normalized persistence and stored procedures for media profiles, jobs, operations, phases, and capability snapshots.
  - The repository enforces stored-procedure-only runtime access and bans JSONB application state.
- Decision:
  - Add migration `0123_media_transcoding_foundation.sql` introducing normalized media tables and first-pass procedures.
  - Add `revaer-data` media modules (`profiles`, `jobs`, `capabilities`) to call those procedures with strongly typed Rust inputs/rows.
- Consequences:
  - Positive outcomes: media persistence exists as a stored-proc boundary and is ready for runtime/API wiring.
  - Risks/trade-offs: this is foundation scope; verification/violation/explanation persistence and deeper job orchestration tables still need follow-up slices.
- Follow-up:
  - Extend schema/procs for plan violations/explanations and operation-level audit details.
  - Wire runtime facade and API handlers to these calls.

## Task Record

- Motivation:
  - Provide a deterministic, policy-compliant DB substrate for the media transcoding feature set.
- Design notes:
  - Added `media_profile`, `media_target`, `media_job`, `media_job_phase`, `media_job_operation`, and `media_capability_snapshot` with explicit columns and relational keys.
  - Added procedures: `media_profile_upsert_v1`, `media_profile_list_v1`, `media_job_create_v1`, `media_job_phase_append_v1`, `media_capability_snapshot_record_v1`, and `media_job_list_v1`.
  - Rust modules call only stored procedures; no runtime raw SQL outside the data layer.
- Test coverage summary:
  - Added schema tests verifying media tables/procedures exist.
  - Added procedure caller tests for profile upsert/list, overlap validation, job create/list/phase append, and capability snapshot recording.
  - Ran `cargo test -p revaer-data media:: -- --nocapture` and `just lint`.
- Observability updates:
  - No new telemetry/events in this slice.
- Status-doc validation:
  - Updated ADR index and docs summary with this task record.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/revaer-data.instructions.md`, and `.github/instructions/rust.instructions.md`.
  - No policy contradictions introduced.
- Risk & rollback plan:
  - Rollback by reverting migration `0123` and `revaer-data/src/media` module additions.
- Dependency rationale:
  - No new dependencies introduced.
