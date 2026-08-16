# Explicit media job re-plan contract

- Status: Accepted
- Date: 2026-08-16
- Operator approval: Explicitly approved by the operator on 2026-08-16:
  "I approve the ADRs as they are now and I'm resuming your goal."

## Problem

- Accepted ADR 449 requires an explicit operator action when a job must use a
  different capability run. It does not decide whether re-plan mutates a job or
  creates a successor, who may request it, what evidence remains, or which ADR
  448 checkpoints become invalid.
- Reusing retry for this action is misleading: retry promises the same immutable
  policy, target, source, capability binding, and plan semantics.
- Silently replacing a binding or selected plan would make prior attempts and
  audit evidence impossible to interpret.

## Options

1. **Mutate the existing binding and plan.** This keeps one visible record but
   destroys immutable provenance and is incompatible with ADR 449.
2. **Create a new plan generation on the same job for capability-only re-plan.**
   Preserve source, policy, target, old plans, and attempts; bind the selected
   completed run to a new generation and execute through a new attempt.
3. **Always create a successor job.** This provides the strongest separation but
   fragments one operator intent even when only the host tool closure changed.

## Recommendation

- Adopt option 2 for capability-only re-plan. Use option 3 when source aggregate,
  profile configuration, policy, desired target, managed-root bindings, or
  destructive intent changes.
- Expose `POST /v1/media/jobs/{media_job_public_id}/replan` only to an
  authenticated principal with a dedicated `media.jobs.replan` authorization.
  Worker, scheduler, discovery, retry, and capability-refresh code cannot call
  this operator command implicitly.
- The request contains:
  - `expected_job_version`, a positive optimistic-concurrency value;
  - `capability_run_public_id`, identifying one completed run explicitly;
  - `reason`, trimmed UTF-8 of 1 through 512 bytes;
  - `confirmation`, exactly `replan` after no normalization beyond surrounding
    ASCII whitespace removal;
  - an idempotency key of 16 through 128 ASCII letters, digits, `-`, or `_`.
- The response returns the same job public id, prior and new plan generation,
  new attempt number, selected capability binding public id, invalidated
  checkpoint count, actor, audit timestamp, and updated job version. Replaying
  the same idempotency key and byte-identical request returns the same response;
  reusing it with different input is a conflict.

### Eligibility And Atomic Transition

- Re-plan is permitted only when all of these are true in one stored-procedure
  transaction:
  - the expected job version is current;
  - the source aggregate and all five job root bindings remain exactly the
    immutable values already captured;
  - the policy, desired target, effective-policy identity, and destructive
    intent remain unchanged;
  - no current claim, native process, cleanup claim, aggregate lease, or root
    recovery ownership can still write for the job;
  - every replacement transaction is durably finalized or rolled back and every
    terminal outbox row for the current generation is reconciled;
  - the job has not completed a verified source replacement;
  - the selected capability run is complete, differs from the current binding,
    and passes ADR 519 identity validation.
- The procedure locks the job, records the command and actor, creates the next
  immutable plan generation, binds the selected capability run, creates the
  next immutable attempt with cause `operator_replan`, invalidates prior
  checkpoint eligibility, increments job version, and appends one audit event
  atomically.
- A re-plan attempt does not consume the retry budget. A later retry of that
  attempt reuses the new plan generation and binding. Re-plan numbering and
  retry numbering remain independently visible in audit evidence.
- Accepted ADR 512 and this ADR use one attempt-creation rule: a new attempt
  number is created only by explicit retry after a terminal failure or by the
  accepted ADR 520 `operator_replan` transition; resuming an attempt only
  advances claim generation.

### Prior Evidence And Checkpoint Invalidation

- Prior capability bindings, plans, selected and rejected reasons, operations,
  commands, measurements, verification checks, artifacts, attempts, and audit
  facts remain immutable and queryable under their original plan generation.
- No prior execution checkpoint is reusable by the new plan generation. A
  changed capability closure invalidates command materialization, native output,
  analysis, inspection, verification, and downstream dependants under accepted
  ADR 448 rules.
- Source aggregate manifests and hashes remain historical evidence, but the new
  attempt revalidates the complete aggregate before planning and again before
  mutation. It does not treat a prior observation as current authority.
- Invalidation rows record prior checkpoint public id, old and new plan
  generation, bounded reason `capability_replan`, actor, and timestamp. Artifact
  bytes follow approved workspace retention; invalidation never deletes them
  synchronously.
- If an operator wants a new policy, target, root binding, source observation, or
  mutation mode, the API rejects re-plan with `media_replan_successor_required`.
  Discovery or an explicit create command creates a new job linked by
  `supersedes_job_public_id`; it never rewrites this job.

### Stable Outcomes

- Use stable codes for `media_replan_forbidden`, `media_replan_confirmation`,
  `media_replan_version_conflict`, `media_replan_active_ownership`,
  `media_replan_replacement_unreconciled`, `media_replan_completed`,
  `media_replan_capability_invalid`, `media_replan_same_capability`,
  `media_replan_source_changed`, `media_replan_successor_required`, and
  `media_replan_idempotency_conflict`.
- Failure changes no binding, plan, attempt, checkpoint eligibility, or job
  version.

## Consequences

- Operators can recover a queued or failed job after a host-tool change without
  losing its original intent or evidence.
- Capability-only re-plan remains one job, while semantic configuration changes
  create a clearly linked successor.
- Conservatively invalidating every prior execution checkpoint repeats work, but
  it satisfies ADR 448's requirement that capability identity match before
  reuse.
- A dedicated authorization and confirmation add friction appropriate to an
  action that changes executable provenance.

## Implementation Boundary

- This accepted ADR authorizes only the command surface, authorization, request
  bounds, same-job capability-only generation, successor rule, eligibility,
  atomic evidence, checkpoint invalidation, idempotency, and stable outcomes
  described above.
- Accepted ADRs 448, 449, 500, 501, and 519 remain binding. This command validates
  and binds executable closures only through ADR 519.
- This ADR does not authorize automatic re-plan, silent latest-run selection,
  policy or target mutation, checkpoint reuse across capability identity,
  completed-job replay, or unrelated behavior from ADRs 507-516. It preserves the
  exact-value holds in ADRs 515 and 516.
- Schema, API, UI, YAML, runtime, workflow, and generated-contract changes must
  remain limited to the accepted contract above.

## Validation

- Decision validation to date is documentation-only.
- During implementation, test authorization, confirmation, every input bound,
  optimistic concurrency, idempotent replay, conflicting idempotency, and
  concurrent re-plan requests.
- Test every ownership and replacement phase, completed jobs, changed source,
  changed root, same and incomplete capability runs, and a valid host upgrade.
- Prove prior evidence remains byte-for-byte readable, every old checkpoint is
  ineligible, the retry budget is unchanged, and retry uses the new binding.
- Add API/OpenAPI/UI flows that clearly distinguish retry, re-plan, and successor
  creation and require operator confirmation.
- An accepted implementation is not complete until focused database/API/E2E
  tests, `just ci`, and `just ui-e2e` pass.

## Follow-up

- Implement accepted ADR 519 and this ADR through one plan-generation schema. ADR
  512 uses the same retry and `operator_replan` attempt-creation rule recorded
  above.
- Add read APIs that key all planning and attempt evidence by plan generation
  before exposing the command in the operator UI.

## Task Record

- Motivation:
  - Complete the explicit re-plan semantics required by accepted ADR 449.
- Design notes:
  - Capability-only evolution stays on one job; semantic intent changes require
    a successor job.
- Test coverage summary:
  - The ADR-only change added no schema, API, UI, or runtime tests.
- Observability updates:
  - Future metrics may use bounded outcome and invalidation-reason enums. Actor,
    job, run, plan, attempt, and idempotency identifiers must not be labels.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, accepted ADRs 448, 449, 484, 500, and 501,
    and accepted ADRs 507-516 and 519. This ADR imports no unrelated state model
    and preserves unresolved exact-value holds.
- Risk & rollback plan:
  - Any reversal requires a superseding ADR. A later rollback must preserve all
    plan generations and refuse writes it cannot represent.
- Dependency rationale:
  - No new dependency is required; existing authentication, stored-procedure,
    idempotency, and audit patterns are sufficient.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/revaer-data.instructions.md`, and
    `.github/instructions/revaer-ui.instructions.md`; no drift or relaxation was
    found.
