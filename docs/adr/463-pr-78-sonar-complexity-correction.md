# ADR 463: PR 78 Sonar Complexity Correction

- Status: Proposed
- Date: 2026-08-13
- Operator approval: Pending.
- Context:
  - Sonar reports cognitive complexity 18 for `verify_plan`, above the enforced limit of 15.
  - The validator's category-first error precedence and exact diagnostics are observable behavior and must remain unchanged.
  - This is a nonarchitectural corrective task record. Its final status semantics await the operator's decision on proposed ADR461; the code correction itself makes no architectural choice.
- Decision:
  - Extract the existing operation-scope predicates into private pure functions.
  - Preserve validation order, accepted inputs, and exact error strings.
  - Add focused public-behavior tests for scope categories that lacked direct coverage.
- Consequences:
  - `verify_plan` remains a linear validation sequence below the cognitive-complexity limit.
  - The helper predicates expose no new API and add no runtime dependency.
- Follow-up:
  - Confirm the current PR 78 Sonar analysis accepts the corrected complexity after the stack update is performed separately.

## Task Record

- Motivation:
  - Restore PR 78's strict Sonar coverage gate without suppressing the finding or changing behavior.
- Design notes:
  - Predicate extraction retains category-first validation rather than switching to operation-first validation, which could change the diagnostic returned for a mixed invalid plan.
- Test coverage summary:
  - Added focused tests for subtitle embedding, subtitle extraction, scoped no-op, scoped container operations, and mixed no-op plans.
  - Ran the media-core verifier tests and repository formatting, check, and lint gates.
- Observability updates:
  - No logging, tracing, metrics, or runtime event surfaces changed.
- Status-doc validation:
  - Rechecked the media status documentation; this corrective refactor changes no capability or readiness claim.
- Risk & rollback plan:
  - The risk is changed validation precedence. Focused exact-error tests and the unchanged category ordering constrain it; rollback is removal of the helper extraction and its tests.
- Dependency rationale:
  - No dependency was added or changed.
- Stale-policy check:
  - Reviewed `AGENTS.md` and `.github/instructions/rust.instructions.md`.
  - No policy drift was found, and no instruction change is required for this corrective refactor.
