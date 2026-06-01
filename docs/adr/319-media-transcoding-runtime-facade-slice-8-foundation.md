# Media transcoding runtime facade slice 8 foundation

- Status: Accepted
- Date: 2026-05-23
- Context:
  - Media persistence procedures now exist in `revaer-data`, but runtime consumers need a stable facade boundary similar to existing torrent runtime storage.
- Decision:
  - Add `revaer-runtime::media::MediaStore` with typed methods for profile upsert/list, job create/list/phase append, and capability snapshot recording.
  - Keep this facade as a thin delegation layer over stored-procedure callers in `revaer-data::media`.
- Consequences:
  - Positive outcomes: workers/API can consume media persistence through `revaer-runtime` without data-layer module coupling.
  - Risks/trade-offs: no orchestration policy in this layer yet; execution semantics remain to be added in later slices.
- Follow-up:
  - Wire API handlers and runtime workers to this facade.
  - Extend facade once verification/violation/explanation persistence tables are added.

## Task Record

- Motivation:
  - Preserve a narrow runtime integration contract while media features are built out.
- Design notes:
  - Added `crates/revaer-runtime/src/media.rs` and exported it from crate root.
  - Methods return `DataResult` and avoid embedding policy logic.
- Test coverage summary:
  - Added `media_store_round_trips_profiles_jobs_and_capabilities` runtime integration test (skips when test DB unavailable).
  - Ran `cargo test -p revaer-runtime -- --nocapture` and `just lint`.
- Observability updates:
  - None in this slice.
- Status-doc validation:
  - Updated ADR index and docs summary.
- Stale-policy check:
  - Reviewed `AGENTS.md` and `.github/instructions/rust.instructions.md`; no contradictions introduced.
- Risk & rollback plan:
  - Revert `revaer-runtime/src/media.rs` and `lib.rs` export to roll back this slice.
- Dependency rationale:
  - No new dependencies introduced.
