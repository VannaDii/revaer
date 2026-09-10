# Stack boundary reconciliation

- Status: Recorded
- Date: 2026-09-10
- Operator approval: Not applicable: nonarchitectural task record
- Context:
  - GitHub's displayed PR sizes and branch names do not prove a linear,
    reviewable stack. ADR 564 identified an unverified PR 99 transition.
- Decision:
  - Audit all current media PR boundaries and correct the isolated PR 99
    reconstruction. Preserve original refs and remote metadata pending full
    descendant reconciliation. Do not change review or quality criteria.
- Consequences:
  - The local PR 99 schema regression is corrected, but no remote repair,
    complete stack validation, source push or merge is claimed.
- Follow-up:
  - Propagate the existing dependency/security and instruction corrections at
    their earliest owning boundaries, repair the phase read-model descendant,
    reconstruct the approved init slices, and reconcile formal membership only
    after verifying the complete ordered branch mapping.

## Verified Boundaries

Read-only CLI/API snapshots on 2026-09-10 cover all 104 open PRs with
`stack/media3-` heads. Their base names form one unbranched chain from `main`;
all 104 titles are conventional. Exact commit ancestry holds at 103 boundaries,
with PR 98 the sole exception. GitHub has two formal stacks: stack 204 contains
41 PRs from 195 through 97, and stack 202 contains 63 PRs from 98 through 194.
No PR is missing formal membership. The local extension does not track the
primary checkout's current branch; that does not mean remote membership is
absent. No browser was used for monitoring or inspection.

The canonical `just stack-changed-lines` gate was run for all 104 current
base/head pairs. It rejects three boundaries:

| PR | Exact Base / Head | Required Repair |
| --- | --- | --- |
| 98 | `54157ef8` / `7ed723d5` | Non-ancestor base. GitHub's comparison shows 131,057 changed lines; the direct endpoint no-rename comparison is 14,940. Neither is a permitted review boundary. |
| 130 | `6471074e` / `a3797174` | 213 binary deletions, 21,064,113 original bytes. The guard fails closed on uncountable entries despite GitHub showing 2,481 text changes. No binary exception is approved or implemented here. |
| 186 | `6eb56bc2` / `adb64ca5` | 97,202 no-rename changed lines, although GitHub reports 9,906. Moving the init path contributes 43,694 deleted plus 43,752 added lines. Reconstruct using the approved stable-path assembly/cutover contract; do not exploit rename detection. |

Other size gates pass at the inspected SHAs. This is not a claim that their
content, approvals, checks or reviews are complete. The private audit retains
full API snapshots, ordered edges, each guard's real status/output, and an
immutable old-blob/byte inventory for the binary deletions.

## PR 99 Correction

The isolated reconstruction at `ccf8b3ef` mistakenly restored the expected
`MediaJobPhaseAppendRequest` schema after repaired base `f87bea0b` removed it.
The focused schema test reproduced the failure. Corrected commit `377a082e`
removes the stale expectation and asserts absence without changing any runtime
schema or route. Its diff against `f87bea0b` is 1,368 changed lines.

Independent review found no other semantic mismatch: 14 of 17 changed files
have identical changed lines to the original PR; the remaining differences are
the corrected schema expectation, necessary import context and documentation
ordering. Descendant `28f72711` reintroduces the append schema while exposing
read-only phases; reconcile that intermediate boundary rather than remove the
new regression assertion. A later removal in `1a0b63e7` is not evidence that
every intermediate PR passes.

## Task Record

- Motivation:
  - Make stack reconstruction evidence-based and preserve each reviewable
    deliverable without forcing an oversized integration history onto GitHub.
- Design notes:
  - Keep original and corrected local refs separate. All remote inspection is
    read-only and exact-SHA based; no stack linking, bypass, force-push or
    required-check mutation occurred. The API assertion preserves the earlier
    API boundary, not a new architecture choice.
- Test coverage summary:
  - Eight OpenAPI tests pass under minimal and all features. Strict API
    all-target Clippy and workspace formatting pass. Full `just ci` reaches
    audit and fails on vulnerable `h2` 0.4.12 and yanked `chacha20` 0.10.0.
    Full `just ui-e2e` stops at the old npm graph's security audit before
    browsers start. These gates remain failed; no criterion was relaxed.
  - Initial local system-Ruby and Node selection failures were corrected using
    the available modern Ruby and NVM-managed exact Node, then both gates were
    rerun. The independent existing-data proof is recorded in ADR 582.
- Observability updates:
  - Retain private guard, GitHub, compiler, test and audit output. No production
    logging, telemetry or scanner settings change.
- Status-doc validation:
  - Update ADR indexes, the generated catalogue and the completion ledger.
    Keep implementation evidence separate from remote and package readiness.
- Risk & rollback plan:
  - A local reconstruction could erase descendant changes if published without
    reconciliation. Keep it unpublished, retain original refs, and validate
    exact repaired boundaries before any conditional push. Discard only the
    owned reconstruction ref to roll back this preparation.
- Dependency rationale:
  - No dependency added. The audit uses existing Git, GitHub CLI, Ruby standard
    libraries and the canonical changed-line gate.
- Stale-policy check:
  - Reviewed root AGENTS, Rust/data/devops scoped instructions, ADRs 522/559/564,
    the task template and the historical PR 99 instructions. Historical scoped
    precedence drift is already corrected in the integration and remains a
    restack propagation requirement. No consent or clean-gate claim is inferred
    from the historical task records.
