# Media Data Target Fail-Closed Validation

- Status: Accepted
- Date: 2026-07-22
- Context:
  - The media graph can inspect opaque data streams so remux and preservation decisions can account for them.
  - Desired target rows do not yet have a verified contract for intentionally selecting, transforming, or validating opaque data streams.
  - Attachment and chapter target rows were already fail-closed; data rows needed the same explicit boundary.
- Decision:
  - Reject `StreamKind::Data` in desired target stream validation with `UnsupportedDesiredStreamKind`.
  - Keep passive inspection and preservation of source data streams available through unmatched-stream policy behavior.
  - Do not add a partial data-stream policy until there is a complete desired-state schema and verification contract.
- Consequences:
  - Operators cannot accidentally express unsupported data-stream requirements as if they were enforced.
  - Existing source-preservation flows keep accounting for data streams without claiming data-specific rewrite support.
  - Future data-stream policy work must introduce validation, planning, execution, and verification together.
- Follow-up:
  - Add a complete data-stream desired-state contract before enabling explicit target rows for opaque streams.

## Task Record

- Motivation:
  - Close a PR 31 media-transcoding correctness gap where explicit opaque data target rows could pass validation despite lacking a verified implementation contract.
- Design notes:
  - Target validation now treats `Data` like `Attachment` and `Chapter`.
  - Runtime inspection remains unchanged; the boundary is only for explicit desired target rows.
  - The change is fail-closed and does not relax any Sonar, lint, security, or coverage criteria.
- Test coverage summary:
  - Added unit coverage in `crates/revaer-media-core/src/target.rs` for rejecting `StreamKind::Data` target rows.
- Observability updates:
  - No runtime logging or metrics changes; invalid target rows already surface through existing compile errors.
- Status-doc validation:
  - `docs/adr/index.md` and `docs/SUMMARY.md` were updated for this task record.
- Risk & rollback plan:
  - Risk is limited to callers that were relying on unsupported explicit data target rows.
  - Roll back this commit if a fully verified data-stream target contract lands in the same replacement change.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/devops.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md`.
  - No stale or contradictory policy text was found.
