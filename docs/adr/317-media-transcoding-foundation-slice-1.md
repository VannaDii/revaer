# Media transcoding foundation slice 1

- Status: Accepted
- Date: 2026-05-23
- Context:
  - `MEDIA_TRANSCODING.md` defines a large multi-slice implementation with strict deterministic and policy requirements.
  - The repository needed a first, policy-clean foundation for media domain logic and runtime adapters before data/API/UI orchestration can be layered in.
- Decision:
  - Add two new crates: `revaer-media-core` for pure deterministic media logic and `revaer-media-runtime` for runtime-facing adapter contracts and execution argument building.
  - Keep the first slice narrow: domain types, normalization, semantic classification, profile validation, diffing, planning, verification, explanation, capability/workspace models, and shell-free ffmpeg argv construction.
- Consequences:
  - Positive outcomes: establishes compile-time boundaries and testable primitives required by later slices.
  - Risks/trade-offs: this slice does not yet deliver full end-to-end transcoding persistence/API/UI behavior; subsequent slices must integrate data procedures, handlers, and feature UX.
- Follow-up:
  - Implement slices 2-15 from `MEDIA_TRANSCODING.md` incrementally on this branch.
  - Add media persistence schema/procedure coverage and runtime orchestration wiring next.

## Task Record

- Motivation:
  - Create a deterministic, non-panicking media foundation that satisfies repo policy and unblocks the rest of the transcoding roadmap.
- Design notes:
  - `revaer-media-core` contains only pure logic and data models; no filesystem/env/process/database behavior.
  - `revaer-media-runtime` defines runtime-side models/contracts and produces argv vectors directly instead of shell command strings.
  - Validation and workspace checks return typed errors for explicit failure semantics.
- Test coverage summary:
  - Added unit coverage for normalization aliases/whitespace, semantic role inference, profile path validation, diff behavior, plan generation, plan verification, capability validity, workspace reserve enforcement, and argument-construction error handling.
  - Reran `cargo test -p revaer-media-core -p revaer-media-runtime` and `just lint`.
- Observability updates:
  - No new runtime logs/metrics/events in this slice; later orchestration slices will add event and telemetry integration.
- Status-doc validation:
  - Verified `MEDIA_TRANSCODING.md` implementation-slice requirements for the foundation scope and updated ADR index/book summary entries.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - No instruction contradictions were introduced in this slice.
- Risk & rollback plan:
  - Risk is limited to new isolated crates and workspace wiring.
  - Rollback path is straightforward: revert this slice commit to remove media crates from workspace membership.
- Dependency rationale:
  - No new third-party dependencies were introduced beyond workspace-standard `serde` and `thiserror`.
  - Alternatives considered: fully custom error/display handling without `thiserror`; rejected to preserve consistency with existing crates.
