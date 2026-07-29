# ADR 468: PR 99 Native Watcher Test Stability

- Status: Proposed
- Date: 2026-08-13
- Operator approval: Pending; this is a nonarchitectural corrective task record awaiting ADR 461 status semantics.
- Context:
  - The native watcher integration test depends on the host operating system delivering an event from a temporary directory after registration.
  - The backend can register successfully without delivering such events in constrained local and hosted environments, so the test can time out without identifying a production-code defect.
- Decision:
  - Extract the existing native callback body into a private function without changing its behavior.
  - Test native recursive registration as a host-backend smoke boundary.
  - Test the exact callback-to-buffer path deterministically with a recursive media path and profile identity.
- Consequences:
  - Registration failures and callback forwarding regressions remain independently observable.
  - Test success no longer depends on nondeterministic host event delivery.
- Follow-up:
  - Verify repeated minimal-feature runs and current-head GitHub checks.

## Task Record

- Motivation:
  - Local PR 99 validation reproduced `deadline has elapsed` after the OpenAPI lint correction passed.
- Design notes:
  - The production callback delegates to the extracted private function with the same inputs, filtering, buffering, and warning behavior.
- Test coverage summary:
  - Run the complete minimal-feature recipe repeatedly with native registration and deterministic callback assertions.
- Observability updates:
  - None; test behavior only.
- Status-doc validation:
  - No product or operator documentation changes are required.
- Risk & rollback plan:
  - The extraction could accidentally alter callback behavior; strict lint and the deterministic callback test constrain that risk.
  - Roll back the extraction if production behavior differs, then introduce an injectable watcher backend before restoring end-to-end host event delivery coverage.
- Dependency rationale:
  - No dependency added.
- Stale-policy check:
  - Reviewed `AGENTS.md` and `.github/instructions/rust.instructions.md`; no drift or contradiction was found.
