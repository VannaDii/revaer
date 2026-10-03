# Native root writer assertion

- Status: Accepted
- Date: 2026-09-21
- Operator approval: Operator directed active-file locking or change detection,
  abandoning deleted inputs and refusing replacement of changed inputs; then
  affirmed that direction with "Great!" in the 2026-09-21 conversation.
- Supersedes: ADR 550's native external-writer assertion requirement
- Implementation status: Not started

## Approved Decision

Native Linux operation does not require a separate exclusivity assertion file.
Use bounded checks on active files and abandon affected work when the source
changes or disappears. Never replace a source known to have changed.

The operator explicitly rejected proving root exclusivity and directed this
file-oriented approach. The 2026-10-01 reconciliation corrects the subsequent
agent-created approval hold; it does not grant new authority or require a new
deployment component.

## Implementation Scope

1. Do not introduce `/etc/revaer/media-deployment.json`. Keep the trusted root
   catalog, allowed-path boundaries, ownership checks, Revaer coordination,
   durability requirements and recovery protections.
2. For `linux_dedicated_service`, retain dedicated subtree, ancestry, non-overlap
   and descriptor-bound Revaer root lock checks. Remove the separate requirement
   to prove or assert that no other container or service can write. The catalog
   declaration describes intended use, not proof about unrelated software.
3. Hold the source descriptor during processing and use supported cooperative
   locks. Validate source identity and content against the inspected input before
   processing and immediately before replacement. A known modification, deletion,
   replacement at the source path, or inability to validate stops that operation.
4. Filesystem notifications may detect interference promptly where supported;
   they are not a required new subsystem or replacement authority. Deletion or
   change cancels affected work;
   notification loss/overflow requires revalidation. Descriptor/path checks
   remain necessary even without an event. Do not promise instant detection.
5. On interference, do not publish the candidate over the source. Clean up only
   Revaer-owned temporary artifacts, retain recovery evidence needed for any
   partially completed operation, and persist a distinguishable changed/missing
   source outcome. Unrelated jobs continue. Require fresh inspection and planning
   instead of silently retrying replacement against a new source.
6. Retain the existing original-preservation and recovery protocol. Atomic rename,
   watchers and cooperative locks are not conditional compare-and-swap against
   an uncooperative writer. Operators must avoid concurrent external writes
   during replacement; the race after the final check is a documented limitation.

## Benefits And Limitations

- Removes a deployment file that records intent without preventing interference.
- Detectable interference becomes a job outcome, not a global configuration or
  dry-run blocker. Other path, permission and safety failures remain enforced
  for the operations they affect.
- Linux advisory locks do not prevent unrelated unlink/rename operations. An
  open descriptor preserves access to the inode, not ownership of its pathname.
  Watchers and final checks cannot eliminate a race after the last check.
- This does not authorize overwriting a known changed source or claim protection
  against hostile administrators. Kubernetes policy, durability classes, catalog
  encoding and package support claims are unchanged.
- Only ADR 550's native external-writer assertion clauses in its sole-writer
  mapping, consequences and approval scope are superseded. Other approved
  safety and qualification requirements continue to apply.

## Validation And Task Record

- Motivation: unblock the real operator workflow without an ineffective new
  declaration requirement. Design: bound interference handling to active files
  and explicitly document the residual race.
- Required real-service tests: deletion, same-path replacement, in-place change,
  notification loss if notifications are used, cancellation/cleanup,
  unrelated-job continuation and source
  changes immediately before publication. Assert original preservation and
  persisted outcomes. These do not prove immunity to the documented final race.
- Milestone evidence remains authenticated UI save, real service restart,
  dry-run planning and unchanged source bytes. Full CI/UI, strict Sonar with
  positive coverage, package qualification and review gates remain mandatory.
- Observability: bounded job outcomes, no paths or file identities in metric
  labels. Dependencies: reuse existing facilities; none added by this proposal.
- Risk/rollback: external writes can race final publication. Document the
  operational limitation. If validation fails, keep replacement unavailable;
  revert implementation without deleting source or recovery artifacts.
- Stale-policy check: reviewed AGENTS.md and ADR 550's native writer clauses.
  Corrected the unnecessary pending-approval classification against the
  operator's explicit direction; optional notifications are not a new gate.
  Production behavior and unrelated criteria are unchanged.
- Test coverage summary: documentation revision only; runtime validation pending.
