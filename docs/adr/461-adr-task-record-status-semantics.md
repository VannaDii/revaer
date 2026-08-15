# ADR and task-record status semantics

- Status: Accepted
- Date: 2026-08-13
- Operator approval: Option B approved wholesale by the operator on 2026-08-15: "I approve the ADRs as they are now and I'm resuming your goal."
- Context:
  - `AGENTS.md` requires every task to persist a task record as an ADR.
  - Architectural decisions require explicit operator approval, while routine corrective tasks still need durable records without manufacturing an architectural approval event.
  - Several completed nonarchitectural records used `Accepted` with `Operator approval: Not required`, making the status vocabulary ambiguous.
- Decision:
  - Select Option B: introduce the nonarchitectural `Recorded` status.
  - Reserve `Accepted` for architectural decisions with explicit, decision-specific operator approval and dated evidence.
  - Permit completed nonarchitectural corrective task records to use `Recorded` with `Operator approval: Not applicable: nonarchitectural task record`.
  - Reclassify affected nonarchitectural records as they enter the replayed stack and audit the final catalogue for consistency.
- Consequences:
  - Architectural authority remains explicit and cannot be inferred from task completion.
  - Routine corrective evidence can describe its completed lifecycle without creating an operator approval bottleneck.
  - Root policy, the ADR template, and existing records must remain aligned with the approved vocabulary.
- Follow-up:
  - Reclassify ADR454, ADR458, ADR459, and other completed nonarchitectural task records during stack replay.
  - Add a final catalogue audit for status and operator-approval field consistency.
  - Do not change production behavior as part of status-only corrections.

## Implementation Boundary

- This approval authorizes the `Recorded` vocabulary, the root policy and template updates, and status-only reclassification of nonarchitectural task records.
- It does not authorize classifying an architectural decision as nonarchitectural or marking an unimplemented task complete.

## Task Record

- Motivation:
  - Remove ambiguity between architectural acceptance and completed corrective task evidence.
- Design notes:
  - The vocabulary separates decision authority from task completion while retaining one durable ADR catalogue.
- Test coverage summary:
  - Validate the documentation build, catalogue links, and final status/approval-field audit.
- Observability updates:
  - None.
- Status-doc validation:
  - `docs/adr/index.md` and `docs/SUMMARY.md` expose this accepted decision.
- Risk & rollback plan:
  - The risk is misclassifying architecture as routine work. Roll back status-only edits and restore `Proposed` if classification is uncertain.
- Dependency rationale:
  - No dependency is added.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `docs/adr/template.md`, and the affected ADR status fields.
  - Drift was found in the former use of `Accepted` for nonarchitectural task records and is corrected by the approved vocabulary.
