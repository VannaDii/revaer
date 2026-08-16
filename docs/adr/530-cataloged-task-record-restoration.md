# Cataloged task-record restoration

- Status: Recorded
- Date: 2026-08-16
- Operator approval: Not applicable: nonarchitectural task record.
- Context:
  - PR 97 catalogs ADRs 378, 379, 500, and 501, but the branch does not contain
    those files. Every descendant therefore fails the documentation link gate
    before its own deliverable can be evaluated.
  - ADRs 379, 500, and 501 record the operator's explicit 2026-08-16 approval.
    ADR 378 is a nonarchitectural task record.
- Decision:
  - Restore the four cataloged records immediately above PR 98 without changing
    their approved boundaries or any production behavior.
  - Keep this correction as one small prerequisite PR so each descendant has a
    self-contained, link-valid base.
- Consequences:
  - Documentation validation can run independently on every later PR.
  - The stack gains one documentation-only prerequisite deliverable.
- Follow-up:
  - Rebase PR 98 and all descendants onto this prerequisite before remote checks
    are treated as integration evidence.

## Task Record

- Motivation:
  - Repair the inherited stack boundary so every PR can pass its required checks.
- Design notes:
  - File contents are restored from the reviewed local records. No code, workflow,
    scanner setting, or architectural criterion changes.
- Test coverage summary:
  - Run the documentation index, link, and build gates, then the repository policy
    and instruction-drift checks.
- Observability updates:
  - None; this change affects documentation integrity only.
- Status-doc validation:
  - `docs/adr/index.md` and `docs/SUMMARY.md` include this task record and the four
    restored records.
- Risk & rollback plan:
  - Risk is limited to stack topology and documentation. Revert this prerequisite
    only after moving every catalog link and approved record to an equivalent
    ancestor.
- Dependency rationale:
  - No dependency was added.
- Stale-policy check:
  - Reviewed `AGENTS.md` and `.github/instructions/devops.instructions.md`.
  - Drift found: the parent catalog referenced four absent records.
  - Contradictions removed: none; this restores the exact referenced contracts.
