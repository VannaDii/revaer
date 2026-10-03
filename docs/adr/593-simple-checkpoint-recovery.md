# Simple checkpoint recovery

- Status: Accepted
- Date: 2026-10-02
- Operator approval: Operator confirmed restart of unfinished steps with temporary
  outputs overwritten, originals untouched until verified final replacement,
  then directed: "Now let's supersede ADR-588 and do a quick check for any
  existing approvals that would result in overly complicated implementations."
- Supersedes: ADR 588's S2/LIFE-1 custom supervisor and recovery machinery,
  dependent ADR 589 registration protocol, and parallel-agent execution direction.
  Retains ADR 588's unrelated decisions and transfer scopes as listed below.
- Implementation status: Decision recorded; product code has not been rolled back
  or qualified under this decision.

## Context

Recovery needs to finish interrupted media work without damaging originals.
The custom owner/controller protocol made that straightforward workflow depend
on substantial infrastructure without establishing that it was necessary.

## Decision

Restart the unfinished step from the last completed checkpoint. Treat that
step's Revaer-owned temporary outputs as untrustworthy and overwrite or discard
them. Leave original media untouched until the verified final replacement step.
Record failures and move on. Interrupted final replacement must determine which
replacement action completed before continuing; it is not temporary-file replay.

Do not require the ADR 588 custom PID1 owner/controller, fixed binary handshake,
identity registry, settlement-receipt protocol, exact deadline partitions, or
external termination evidence submitted by an operator to resume routine work.
Ordinary child-process cancellation and shutdown remain required; stop the old
work before restarting it. Do not introduce a replacement coordination framework.
Keep existing process deadlines and resource bounds unless separately changed.

Work single-agent and outside-in. The next product deliverable remains the real
authenticated configuration/restart/dry-run workflow, followed by execution,
verification, replacement and recovery.

### Retained Decisions

- ADR 588 choices 1 and 2: scoped ingestion fixes and verified package startup.
  ADR 591's unreleased single-init decision supersedes historical migration and
  legacy-parity requirements; do not resurrect them.
- Choice 4: discovery functionality and applicable limits remain approved subject
  to the accepted simplifications in [ADR 594](594-simple-discovery-worker-cleanup.md).
- Choice 5: three audio intents and their acceptance requirements.
- Choice 6: only the original content-bound, expiring asset-removal exception.
- Choice 7: existing scoped GitHub, Sonar and release-artifact transfer permission,
  with its original exclusions, destinations and revocation conditions.
- Dry-run default, source-change handling under ADR 592, stored-procedure-only
  persistence, strict quality gates, positive Sonar coverage, real failure-path
  evidence, supported packages, PR review and stack-size requirements.

Historical ADR 588 appendices are evidence, not current supervisor requirements.
Where they conflict with this decision, this decision controls. Unrelated
approval conditions remain binding; supersession is not release qualification.

## Quick Approval Check

This is a focused review, not a claim to have audited every historical ADR.
The discovery/worker/cleanup recommendations below were subsequently approved
through ADR 594 on 2026-10-02; their pending wording is historical.

| Approval | Finding and recommendation | Current disposition |
| --- | --- | --- |
| 588 S2/LIFE-1 and 589 | Custom owner/controller and registration protocol are unnecessary for the chosen recovery model. Remove the isolated implementation in a subsequent scoped change. | Superseded here. |
| 588 DISC-1 through DISC-7 | Directory-epoch/name-frontier persistence, principal/deployment byte-debit windows and slow-storage timing assumptions add coordination beyond ordinary discovery. Recommend a simpler saved scan position plus rescan after uncertainty, keeping source identities and bounded processing. | Recommendation only; existing approval remains until an exact discovery delta is approved. |
| 512 | Separate attempt/aggregate leases and per-root recovery leadership can turn local restart into distributed coordination. Recommend retaining job checkpoints and one active replacement per source without a new distributed recovery framework. | Recommendation only; not globally superseded. Custom 588 supervisor requirements no longer apply. |
| 513 | Per-candidate cleanup claims and takeover timing add another durable lifecycle. Recommend protecting active/checkpoint/replacement files and retrying failed cleanup individually without an independent cleanup-owner protocol. | Recommendation only; not globally superseded. |
| 590 and 591 | Typed configuration, immutable referenced versions, disabled/dry-run defaults and outside-in single-init delivery directly serve the operator workflow. | Retain; no complexity-driven change proposed. |
| 592 | Change/loss detection instead of proof of exclusive root ownership already follows the pragmatic boundary. | Retain. |

## Consequences And Follow-up

Less custom infrastructure and a shorter critical path. Incomplete steps may
repeat work; that is an acceptable recovery cost. This does not promise recovery
from arbitrary external interference or remove final-replacement safety.

Next remove only the isolated supervisor detour, preserving useful workflow work
and user changes. Present exact discovery/worker/cleanup deltas before changing
their remaining approved contracts. Do not use that review to block unrelated
operator-workflow implementation or launch broad database investigations.

## Task Record

- Motivation: replace overcomplicated recovery machinery with the operator's
  checkpoint-and-rebuild decision.
- Design notes: temporary outputs may be rebuilt; originals change only at final
  verified replacement. Supersession is scoped, not a blanket approval reset.
- Test coverage summary: documentation consistency checked only. No runtime
  tests run and no CI, UI, Sonar or package pass claimed. Implementation requires
  real restart and interrupted-replacement evidence plus all existing gates.
- Observability updates: retain step outcomes and bounded failure reasons;
  no new telemetry system.
- Status-doc validation: ADR index and documentation navigation updated;
  historical implementation evidence remains historical.
- Risk and rollback plan: mixed old/new instructions could revive the detour;
  explicit supersession notices and scoped instruction alignment prevent that.
  No product files removed in this task.
- Dependency rationale: no dependencies added.
- Stale-policy check: reviewed supplied global/root rules and scoped Rust
  instructions. Removed mandatory custom supervisor instructions; retained
  unrelated safety and quality criteria. Corrected ADR 592's stale pending label
  in the navigation to match its accepted record.
