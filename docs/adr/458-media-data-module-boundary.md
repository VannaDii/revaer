# Media data module boundary integration correction

- Status: Accepted
- Date: 2026-08-13
- Operator approval: Not required; this moves an existing module rename to its first required integration boundary and makes no architecture choice.
- Context:
  - The durable job invariant change introduced `crates/revaer-data/src/media.rs`.
  - The next data-access deliverable introduced `crates/revaer-data/src/media/mod.rs` while leaving the flat module in place.
  - Rust rejects a crate containing both module paths, so every intermediate deliverable from data access through job runtime was uncompilable.
  - The job-runtime deliverable later renamed the flat module to `media/identity.rs` and exported its public types.
- Decision:
  - Move that existing rename and export into the data-access deliverable where the directory module first appears.
  - Keep the implementation, public exports, and final leaf tree unchanged.
  - Remove the now-redundant rename from the later job-runtime patch while replaying the stack.
- Consequences:
  - Every data-access descendant has one unambiguous `media` module and can compile independently.
  - The job-runtime patch becomes smaller without changing its runtime behavior.
- Follow-up:
  - Run formatting, checks, and lint at the corrected data-access boundary.
  - Replay descendants and run the complete leaf handoff gates.

## Task Record

- Motivation:
  - The formal stack requires every PR to be independently reviewable and green.
- Design notes:
  - This is patch relocation only; no type, stored procedure, persistence, or runtime contract changes.
- Test coverage summary:
  - Run `just fmt-check`, `just check`, and `just lint` at this boundary, followed by full leaf verification.
- Observability updates:
  - None.
- Status-doc validation:
  - The ADR index and documentation summary are updated; product status is unchanged.
- Risk & rollback plan:
  - Risk is limited to an incomplete module export during history replay. Compilation and linting detect that immediately.
  - Restore the rename to the later commit to roll back, accepting that the intermediate PRs become invalid again.
- Dependency rationale:
  - No dependency is added.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and `.github/instructions/revaer-data.instructions.md`.
  - No policy drift or criteria relaxation was introduced.
