# Operator-approved architecture

- Status: Accepted
- Date: 2026-08-13
- Operator approval: Explicitly approved by the operator on 2026-08-13 when
  directing that agents are not authorized to make architectural decisions
  without operator agreement and approval.
- Context:
  - Architectural choices affect the product contract, system boundaries,
    operational model, and future implementation constraints.
  - Agents may discover and analyze architectural questions, but the operator
    retains decision authority.
  - The prior ADR template did not require evidence of operator approval before
    an ADR was marked accepted or its recommendation was implemented.
- Decision:
  - Agents must stop affected implementation, write and present a `Proposed` ADR,
    and receive explicit decision-specific operator approval before accepting or
    implementing an architectural decision.
  - ADRs record approval explicitly. Pending, inferred, or agent-authored consent
    is not approval.
  - Unapproved architectural implementations remain local and unpushed until the
    operator accepts, revises, or rejects the proposal.
- Consequences:
  - The operator has a durable review gate before architecture changes.
  - Evidence gathering may continue without pre-committing the repository to a
    design.
  - Work can pause at an ADR boundary when operator review is required.
- Follow-up:
  - Apply the approval field to new ADRs.
  - Present all pending architecture ADRs for operator review before associated
    work advances or is pushed.

## Task Record

- Motivation:
  - Record the operator's instruction that agents do not have unilateral
    architectural decision authority.
- Design notes:
  - The rule is placed in root `AGENTS.md` so scoped instructions cannot relax it.
  - The ADR template makes approval state visible and reviewable.
- Test coverage summary:
  - `just instruction-drift` and `just ci` pass.
  - `just ui-e2e` passes all 104 API and Chromium tests.
- Observability updates:
  - None; this is a governance control.
- Status-doc validation:
  - Updated the ADR index and documentation summary.
- Risk & rollback plan:
  - The rule intentionally pauses affected implementation for operator review.
    Rollback requires explicit operator direction because it would remove an
    approval control.
- Dependency rationale:
  - No dependency added.
- Stale-policy check:
  - Reviewed `AGENTS.md` and the ADR template. The missing approval gate was the
    identified policy gap; no scoped instruction may relax the new root rule.
