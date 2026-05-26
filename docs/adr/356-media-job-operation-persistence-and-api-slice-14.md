# Media job operation persistence and API slice 14

- Status: Accepted
- Date: 2026-05-25
- Context:
  - Media job operations are planned/executed concepts in slice 14, but operation rows were not persisted through stored procedures.
  - API surface exposed profile/job/phase/capability behavior but lacked operation append/list endpoints.
- Decision:
  - Add `media_job_operation_append_v1` and `media_job_operation_list_v1` stored procedures.
  - Add `revaer-data` and `revaer-runtime` operation append/list APIs with typed rows.
  - Extend media API/app wiring with `/v1/media/jobs/{media_job_public_id}/operations` GET/POST.
- Consequences:
  - Positive outcomes: deterministic execution intent can now be persisted and retrieved per job.
  - Risks or trade-offs: operation arguments are currently bounded to five positional fields; broader command modeling remains future work.
- Follow-up:
  - Integrate operation persistence into preflight/execution orchestration flow when worker runtime wiring lands.

## Task Record

- Motivation:
  - Continue slice 14 by closing the operation persistence/API gap needed for deterministic execution explainability.
- Design notes:
  - New migration `0129_media_job_operation_procs.sql` adds append/list procedures with upsert-on-index behavior.
  - `revaer-data::media::jobs` now exposes `append_media_job_operation` and `list_media_job_operations`.
  - `revaer-runtime::MediaStore` now exposes `append_job_operation` and `list_job_operations`.
  - API model and handler wiring now supports operation append/list route behavior.
- Test coverage summary:
  - Updated data/runtime tests to write and read operation rows.
  - Re-ran:
    - `cargo check -p revaer-api-models -p revaer-data -p revaer-runtime -p revaer-app -p revaer-api`
    - `cargo test -p revaer-data media::jobs::tests::create_and_list_media_job -- --nocapture`
    - `cargo test -p revaer-api http::handlers::media::tests:: -- --nocapture`
- Observability updates:
  - None.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`; this increment advances slice-14 operation persistence and API coverage.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Low-to-moderate risk due new migration and API routes; rollback is reverting migration + media operation API wiring.
- Dependency rationale:
  - No new dependencies added.
