# Attempt-aware media job phase reads

- Status: Accepted
- Date: 2026-08-16
- Operator approval: Explicitly approved by the operator on 2026-08-16:
  "I approve the ADRs as they are now and I'm resuming your goal."

## Context

- The source-stack ADR 353 (`353-media-job-phase-read-model.md`) added
  `GET /v1/media/jobs/{media_job_public_id}/phases` and
  `media_job_phase_list_v1` when phase rows were treated as one job-level list
  ordered only by `phase_index`.
- Media execution now has immutable attempts and claim-generation fencing. A
  retry can reuse phase indexes without replacing the prior attempt's evidence,
  so a job-level list ordered only by `phase_index` is ambiguous and can present
  phases from separate attempts as one lifecycle.
- Operators have two distinct questions:
  - "What is happening in the current attempt?"
  - "What happened across every attempt, including retries?"
- One response that silently combines those questions is difficult to bound,
  paginate, cache, and explain. It also risks exposing internal attempt UUIDs or
  claim generations that are fencing details rather than operator identifiers.
- Runtime database reads must remain stored-procedure-backed, deterministic, and
  bounded.

## Options

1. **Keep one job-level phase endpoint.** Return every phase from every attempt
   through the existing route, add attempt fields, and order the combined list.
   This preserves one route but makes the common progress read grow with retry
   history and gives the route two incompatible meanings.
2. **Make the existing endpoint current-attempt-only and omit history.** This
   gives operators an unambiguous progress read but removes complete retry
   diagnostics from the public contract.
3. **Separate current progress from attempt history.** Keep the existing route
   for the current attempt and add a distinct, bounded, keyset-paginated history
   route that identifies every attempt explicitly.

## Recommendation

- Adopt option 3 as the outside-in operator contract.
- `GET /v1/media/jobs/{media_job_public_id}/phases` means **the attempt that is
  current when the first page is requested**, not a merged job history. Its
  response includes the public attempt number, ordered phase rows, and an opaque
  continuation cursor. A continuation cursor remains bound to that attempt even
  if a retry becomes current while the client is paging.
- `GET /v1/media/jobs/{media_job_public_id}/phase-history` means **all retained
  attempts**. It returns rows ordered by attempt number descending and phase
  index ascending so the latest attempt is inspected first while each attempt's
  lifecycle remains chronological.
- Both routes default to 50 rows and reject limits above 100. They return an
  opaque next cursor rather than accepting caller-authored offsets or internal
  identifiers. The history cursor is keyed by `(attempt_number, phase_index)`;
  the current-attempt cursor additionally carries the server-resolved attempt
  number.
- The response exposes the stable public attempt number. It does not expose the
  internal attempt row identifier or claim generation.
- A known job with no attempt returns an empty current-attempt page with no
  attempt number. An unknown job returns `404`, and an invalid or mismatched
  cursor returns a stable invalid-request response rather than restarting the
  read silently.
- Separate stored procedures implement the current-attempt and all-attempt
  queries. Each procedure validates the page bound and applies the complete
  keyset predicate before materializing rows.

## Consequences

- The common operator read remains small and has one precise meaning even after
  retries.
- Full retained history remains available without an unbounded response.
- Clients that need historical diagnostics must call the explicit history route
  and render attempt boundaries.
- The existing endpoint's semantics change from an ambiguous job-level list to
  a current-attempt view. Because the product has not shipped, the contract can
  be corrected before compatibility obligations exist.
- Cursor encoding, OpenAPI models, stored procedures, API tests, and operator UI
  labels must be updated together during implementation.

## Implementation Boundary

- This accepted ADR authorizes only the two read contracts, public attempt number,
  ordering, page bounds, cursor binding, and stored-procedure reads described
  above.
- It does not authorize changing attempt creation, claim fencing, cancellation,
  phase append ownership, evidence retention, retry policy, or worker state
  transitions.
- It does not authorize a public phase mutation endpoint, caller-selected
  attempt identifiers, exposure of claim generations, offset pagination, or an
  unbounded compatibility mode.

## Follow-up

- Implement schema, stored procedures, runtime facades, API models, OpenAPI, and
  UI behavior only within the accepted boundary above.
- Prove cursor stability across concurrent phase appends and an
  intervening retry, as well as page bounds, attempt ordering, empty history,
  unknown jobs, and invalid cursors.

## Task Record

- Motivation:
  - Resolve the PR 108-120 replay ambiguity between ADR 353's job-level phase
    list and the approved attempt-fenced execution model before replaying the
    phase-read change.
- Design notes:
  - The recommendation separates operator intent at the route boundary and uses
    public attempt numbers while retaining internal claim generations solely for
    write fencing.
  - Keyset ordering is total because phase indexes are unique within an attempt.
- Test coverage summary:
  - The ADR-only change added no runtime, API, or schema behavior.
  - `just policy`, `just instruction-drift`, `just docs-build`,
    `just docs-link-check`, and `git diff --check` passed.
  - The runtime matrix in Follow-up remains mandatory before any accepted
    implementation can be handed off.
- Observability updates:
  - No telemetry changes are made by this ADR. A future implementation may
    use existing bounded API latency and error telemetry; it must not put job,
    attempt, cursor, or source identifiers in metric labels.
- Status-doc validation:
  - Reviewed ADRs 353, 430, 495, and 500 and `MEDIA_TRANSCODING.md`. This ADR
    corrects a read-contract ambiguity and does not claim the behavior already
    exists.
  - `README.md`, roadmap/status documents, and operator guides were not changed;
    none currently claims this accepted contract is implemented.
- Risk & rollback plan:
  - Acceptance alone changes no production behavior. Any reversal requires a
    superseding ADR.
  - After implementation, rollback must preserve attempt-scoped evidence and may
    restore only a bounded current-attempt read, never the ambiguous unbounded
    merged list.
- Dependency rationale:
  - No new dependency is required. Existing API, cursor, and stored-procedure
    patterns are sufficient.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and
    `.github/instructions/revaer-data.instructions.md` as the prospective
    implementation constraints.
  - No instruction drift or contradiction was found. ADR 353's pre-attempt read
    assumption is architectural history, not current policy, and this ADR
    records the required reconciliation without changing policy.
