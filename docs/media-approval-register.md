# Media Approval Register

Decision navigation as of 2026-09-11, based on retained integration `f38e6a79`
and the later operator approval recorded in ADR 587. Linked ADRs and their exact
operator evidence remain authoritative. This register grants no consent and
does not certify implementation. Read the complete linked contract before work;
conditions and expiries are not discarded by a summary row.

## Recorded Approvals

| Decision | Status And Evidence | Scope / Conditions | Affected Work |
| --- | --- | --- | --- |
| R1-R4, B1-B3, narrow G1 | Approved 2026-09-09, [559](adr/559-media-approval-delta.md#approval-resolution), resolution `e8be2459`. | Exact root and broker contracts in [557](adr/557-root-persistence-contract.md)/[558](adr/558-rvb1-native-process-broker-wire-contract.md); G1 is internal semantics-preserving work only. E1 and other enumerated holds were not released. | L1-L10, especially root binding and native execution. |
| D1, D2 | Approved 2026-09-10, [569](adr/569-init-privilege-and-timeout-resolution.md#approval-resolution), reviewed `1d62d087`. | Exact stock-extension boundary/expiry and function-scoped reset timeout. Final-init proof and cutover gates remain required. | L1. |
| D3 | Conditionally approved in the same [569 resolution](adr/569-init-privilege-and-timeout-resolution.md#approval-resolution). | Independent cold/warm/helper ingestion proof required; not satisfied by shared legacy failures or partial parity. Does not approve D4/D5. | L1. |
| S1 | Approved 2026-09-10, [577](adr/577-runtime-shutdown-event-classification.md#approval-resolution). | Only requested cancellation classification. Unexpected cancellation/panic warnings remain; no watcher termination policy supplied. | Runtime observability. |
| F1 | Narrowly approved 2026-09-10, [578](adr/578-locked-fixture-diagnostic-disposition.md#approval-resolution). | Exact fixture identity, diagnostic and test-preparation contract. Drift expires it; original evidence remains. No production or Sonar exception. | L7/L10 fixture preparation. |
| Evidence-led delivery and record separation | Approved 2026-09-11: "Agreed, make it so", [587](adr/587-evidence-led-delivery-governance.md). | Eight preceding delivery recommendations; new routine records separated from architectural ADRs. All existing quality gates and decision holds remain. | Delivery process and verification planning, not runtime policy. |

Earlier accepted architectural authority remains linked from
[559's retained dependencies](adr/559-media-approval-delta.md#remaining-holds)
and the [completion ledger](adr/564-media-completion-ledger.md). This is not a
retroactive reapproval or an exhaustive historical ADR re-audit.

## Pending Or Held

| Decision / Owner | Status | Exact Choice Still Needed / Impact |
| --- | --- | --- |
| C1 / [586](adr/586-compliance-manifest-failure-boundary.md) | Pending | Typed fatal startup failure for invalid/missing compliance metadata versus an explicitly designed degraded contract. The recommendation prevents the entire service from starting, including unrelated APIs; general delivery approval does not select it. Blocks affected provider/router production wiring. |
| D4 / [579](adr/579-ingestion-policy-temporary-table-lifetime.md) | Pending | Exact final-init `ON COMMIT DROP` correction for the policy work table; changes legacy transaction behavior. Preserve the warm-session counterexample until approved and fixed. |
| D5 / [583](adr/583-ingestion-imdb-conflict-inference.md) | Pending | Exact final-init IMDb conflict predicate correction. Shared reference/final failure is not successful parity. |
| E1 / [558](adr/558-rvb1-native-process-broker-wire-contract.md) | Held | Exact package environment values, paths, HOME ownership and digest with amd64/arm64 evidence. Nine-key isolation approval does not approve candidate bytes or packages. |
| S2 / [577](adr/577-runtime-shutdown-event-classification.md#s2-bound-investigation-hold-retained) | Held | A defensible shutdown bound and cleanup contract. No arbitrary timeout, process-abort fallback or unproved watcher termination. |
| Binary deletion exception / [585](adr/585-fixed-binary-deletion-review.md) | Proposed, not approved | Exact binary boundaries only if separately approved; current canonical size/ancestry/binary gates remain unchanged. |
| Worker recovery and retention / [512](adr/512-fenced-resumable-worker-ownership-and-recovery.md), [513](adr/513-attempt-scoped-workspace-retention-transaction.md), [514](adr/514-packaged-media-subsystem-lifecycle-and-health-contract.md) | Enumerated values held | Evidence-backed leases, heartbeats, recovery, cleanup takeover/expiry and retry/backoff where undecided. Approved exact values elsewhere stay fixed. |
| Audio acceptance / [515](adr/515-versioned-audio-transformation-and-acceptance-contract.md) | Enumerated choices held | Measured presets, operation order and bitrate meaning/tolerances before affected transforms are enabled. |
| Discovery/admission / [516](adr/516-durable-discovery-scheduling-and-versioned-aggregate-identity.md), [535](adr/535-bounded-cancellation-aware-fingerprint-admission.md) | Enumerated choices held | Scheduler/traversal/hash budgets, aggregate encoding/limit semantics and admission values before held behavior is activated. |

## Decision Requests

Present a bounded batch only when each choice is ready: problem and reproduced
evidence, alternatives, recommendation, exact contract delta, blast radius,
rollback, expiry where applicable, and affected L rows. Record the operator's
actual answer and date in the owning ADR, then synchronize this register.
If approval is conditional, retain the condition until evidence satisfies it.

Production behavior, data-loss authority, numerical SLO/resource targets,
support claims and quality criteria cannot be approved through routine task
records. An unavailable test environment, missing service entitlement or absent
review is an execution/access blocker, not an architectural decision or a pass.
