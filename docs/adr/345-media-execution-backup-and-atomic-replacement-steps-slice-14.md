# Media execution backup and atomic replacement steps slice 14

- Status: Accepted
- Date: 2026-05-24
- Context:
  - Slice 14 requires explicit execution support for backup and atomic replacement behavior.
  - The runtime step model only represented command execution and output verification.
- Decision:
  - Extend runtime execution steps with explicit `BackupSource` and `AtomicReplace` stages.
  - Add `build_execution_steps_with_replacement` to build deterministic backup -> execute -> verify -> replace flows with capability checks.
- Consequences:
  - Positive outcomes: execution plans now model destructive-flow boundaries explicitly and are testable as data.
  - Risks or trade-offs: full quarantine/failure recovery orchestration is still pending later slice-14 increments.
- Follow-up:
  - Integrate replacement-step construction into higher-level job orchestration once runtime replacement policy wiring lands.

## Task Record

- Motivation:
  - Continue slice-14 execution implementation with explicit safe-mutation sequencing.
- Design notes:
  - Added new `ExecutionStep` variants: `BackupSource` and `AtomicReplace`.
  - Added `build_execution_steps_with_replacement` with optional backup support.
  - Preserved single-operation composition guard and capability gating.
- Test coverage summary:
  - Added `execute::tests::replacement_steps_include_optional_backup_verify_and_atomic_replace`.
  - Re-ran:
    - `cargo test -p revaer-media-runtime`
    - `cargo clippy -p revaer-media-runtime --all-targets --all-features -- -D warnings`
- Observability updates:
  - None.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`; this increment advances slice 14 execution operations coverage.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Low-to-moderate risk due new step sequencing API; rollback is revert of this ADR-linked code slice.
- Dependency rationale:
  - No new dependencies added.
