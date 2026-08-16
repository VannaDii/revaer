# Media finalized replacement terminal invariant

- Status: Accepted, reconciled to the approved attempt-scoped replacement transaction
- Date: 2026-07-29
- Reconciled: 2026-08-16

## Motivation

- A destructive replacement must never be reported as cancelled after its durable terminal
  transaction has completed.
- The original implementation proposed a second, unfenced completion procedure after filesystem
  finalization. The later attempt-scoped replacement transaction supersedes that mechanism by
  recording completed job state and terminal outbox evidence atomically before backend cleanup.

## Design notes

- `media_job_worker_commit_replacement_terminal_v1` is the only destructive replacement terminal
  transition. It validates the current claim generation, resolves any cancellation already visible
  at the commit boundary, updates the job and attempt together, and creates completion outbox
  evidence before backend finalization begins.
- Once that transaction reports completion, a later cancellation request is rejected because the
  job is terminal. Backend finalization and startup recovery do not perform another state transition.
- This reconciliation implements the operator-approved attempt-fencing and replacement-recovery
  decisions. It does not introduce a new lifecycle choice.

## Test coverage summary

- Added a data-layer regression proving the terminal transaction is idempotent, rejects a later
  cancellation request, preserves completed state, and retains attempt-bound outbox evidence.
- Added a runtime regression that pauses after backend finalization, proves cancellation is already
  rejected, and then observes the completed lifecycle event.

## Observability updates

- No new event or metric is required. The existing attempt-bound completion outbox remains the
  canonical publication evidence.

## Risk and rollback plan

- Risk: moving the durable terminal transaction after backend finalization would reopen a state and
  filesystem divergence window.
- Rollback: revert the regression-only change. Do not restore the superseded unfenced completion
  procedure without a separately approved architectural decision.

## Dependency rationale

- No dependencies were added.

## Stale-policy check

- Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
  `.github/instructions/revaer-data.instructions.md`, and
  `.github/instructions/devops.instructions.md`.
- Drift found: the historical finalized-completion implementation no longer matched the accepted
  attempt-scoped transaction.
- Contradictions removed: the unfenced post-finalization completion procedure and its late-cancel
  acknowledgment contract are not restored.
