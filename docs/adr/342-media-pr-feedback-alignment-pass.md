# Media PR feedback alignment pass

- Status: Accepted
- Date: 2026-05-24
- Context:
  - PR #31 had unresolved review feedback around stored-procedure call conventions, overlap validation correctness, handler-level status validation, capability snapshot persistence guards, and ffprobe CLI invocation shape.
- Decision:
  - Apply a grouped remediation pass across media core/data/api/app/runtime modules:
    - path-segment-aware overlap validation in `revaer-media-core` and migration procedure checks
    - named-argument stored-procedure call strings in `revaer-data::media`
    - latest capability reads via stored procedure `media_capability_snapshot_latest_v1`
    - media status/phase-status validation at API handler boundary
    - capability refresh snapshot validity guard before persistence
    - YAML retention-days validation parity with DB bounds
    - remove unused router default-state build and keep API lint-clean
    - use positional input syntax for ffprobe inspect adapter invocation
- Consequences:
  - Positive outcomes: review feedback is addressed with deterministic behavior and better policy alignment.
  - Risks or trade-offs: this pass does not yet redesign planner semantics for removed streams or multi-operation execution composition; those remain follow-up scope.
- Follow-up:
  - Continue media planner/execute semantic hardening slices for stream-kind-aware planning and operation composition.

## Task Record

- Motivation:
  - Close outstanding PR feedback while keeping implementation momentum on media transcoding slices.
- Design notes:
  - Kept changes localized to existing modules and reused current error-mapping surfaces.
  - Added `media_capability_snapshot_latest_v1` SQL function to align runtime reads with stored-procedure-only policy.
- Test coverage summary:
  - Re-ran targeted suites:
    - `cargo test -p revaer-api media`
    - `cargo test -p revaer-app media`
    - `cargo test -p revaer-data media`
    - `cargo test -p revaer-media-core`
    - `cargo test -p revaer-media-runtime`
- Observability updates:
  - API handler status validation now fails fast with explicit 400 responses.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, `AGENTS.md`, `.github/instructions/rust.instructions.md`; no contradictory policy text changes required.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none for these modules.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Risk is moderate due cross-crate changes; rollback is a single commit revert.
- Dependency rationale:
  - No new dependencies added.
