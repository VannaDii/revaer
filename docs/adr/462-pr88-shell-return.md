# ADR 462: PR 88 Shell Return Correction

- Status: Recorded
- Operator approval: Not applicable: nonarchitectural task record under ADR 461
- Date: 2026-08-13
- Context:
  - Sonar reported that `fixture_size` ended with an implicit shell-function return around `scripts/test-fixtures/lib.sh:12`.
  - The fixture helper must retain the byte-count pipeline output and its exact exit status while satisfying the repository's fail-closed Sonar posture.
- Decision:
  - Return the immediately preceding byte-count pipeline status explicitly with `return "$?"`.
  - Extend the existing fixture-integrity shell test to prove the successful byte count and preservation of a nonzero pipeline status.
  - Treat this as a nonarchitectural corrective task record under the status semantics approved in ADR 461; no architectural decision is introduced.
- Consequences:
  - Sonar can verify an explicit function return without weakening, excluding, or suppressing analysis.
  - Callers continue receiving the same output and success or failure status as the pipeline produced before this correction.
- Follow-up:
  - Require the focused fixture shell suite and repository policy gates to remain green.
  - Confirm the refreshed PR 88 Sonar analysis no longer reports the explicit-return finding.

## Task Record

- Motivation:
  - Repair the current PR 88 Sonar finding while preserving maximal scanner strictness and fixture-helper behavior.
- Design notes:
  - `return "$?"` is adjacent to the pipeline so no intervening command can replace its status.
  - The regression test overrides `wc` only inside a subshell and requires the sentinel status `37` to propagate unchanged.
  - No scanner setting, quality criterion, runtime architecture, or public interface changes.
- Test coverage summary:
  - Run `just test-fixture-scripts` for focused success, failure-status, integrity, download, and probe guardrails.
  - Run the relevant repository policy and instruction-drift recipes.
  - Run `just clean-test-fixtures` after validation and verify ignored media fixture directories are absent.
- Observability updates:
  - No runtime logging, tracing, metrics, health, or event surface changes.
  - The focused test emits a precise failure message if byte output or status propagation drifts.
- Status-doc validation:
  - Rechecked the ADR catalogue and documentation summary; this correction does not alter product capability or operator guidance.
- Risk & rollback plan:
  - The primary risk is accidental exit-status masking; the sentinel failure regression covers that behavior directly.
  - Rollback is removal of the explicit return and its focused assertions, though that would restore the Sonar finding.
- Dependency rationale:
  - No dependency was added or changed; the test uses Bash function scoping and existing repository tools.
- Stale-policy check:
  - Reviewed `AGENTS.md` and `.github/instructions/devops.instructions.md` for task-record, fixture, shell-safety, and Sonar requirements.
  - No policy drift or contradiction was found, so no instruction file change is needed.
