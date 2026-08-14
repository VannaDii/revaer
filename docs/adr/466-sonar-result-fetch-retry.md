# ADR 466: Sonar Published-Result Fetch Retry Correction

- Status: Proposed
- Date: 2026-08-13
- Operator approval: Pending
- Context:
  - The strict Sonar post-scan verifier has separate bounded retries for individual API requests and for publication of the complete result.
  - `fetch_published_result` was invoked as a bare command while `set -e` was active, so an exhausted transient API-request failure terminated the verifier before the configured published-result retry loop could make its next attempt.
  - Aggregate response files persisted between result attempts, allowing a partial fetch to leave stale evidence from an earlier attempt.
  - Exhaustion read every expected response without checking existence or limiting output size.
  - This is a nonarchitectural corrective task record. Its final status semantics await the operator's decision on proposed ADR461; no architectural choice is introduced here.
- Decision:
  - Evaluate `fetch_published_result` inside the published-result loop condition so failed aggregate fetches follow the existing result retry and exhaustion path.
  - Delete all four aggregate response files before each result attempt so partial fetches cannot validate or print stale data.
  - On exhaustion, print at most 4096 bytes per response and emit an explicit unavailable marker when a response file does not exist.
  - Buffer each bounded response excerpt and emit its label and payload in one write so shell-coverage instrumentation cannot interleave with or detach the evidence label.
  - Preserve the existing retry counts, delays, transient HTTP classifications, coverage requirements, quality-gate requirements, unresolved-issue requirement, and hotspot requirement unchanged.
- Consequences:
  - A transient fetch failure can recover within the existing outer result retry budget without weakening fail-closed behavior.
  - Permanent or non-strict results still exhaust deterministically and emit bounded evidence for every expected response.
  - Evidence records retain their labels during both ordinary execution and `kcov` instrumentation.
  - The correction adds no suppression, exclusion, scanner relaxation, or runtime dependency.
- Follow-up:
  - Confirm PR 138's refreshed Sonar job completes strict post-scan verification after this correction is integrated separately.
  - Resolve this record's final status according to the operator-approved ADR461 semantics.

## Task Record

- Motivation:
  - Prevent a transient Sonar result-fetch failure from bypassing the configured publication retry while retaining maximal scanner strictness.
- Design notes:
  - The low-level API retry budget remains independent from the aggregate result retry budget.
  - Strict validation runs only after all four current-attempt responses are fetched successfully.
  - The fixed evidence limit affects diagnostics only and cannot alter pass or fail evaluation.
- Test coverage summary:
  - Added focused shell regressions proving a late transient fetch failure reaches the next outer attempt, per-attempt cleanup removes stale successful responses before a later partial fetch, missing evidence remains printable, oversized evidence is truncated, and the complete suite passes while instrumented by `kcov`.
  - Retained coverage for low-level 503 and 429 retries, delayed strict-result publication, invalid retry configuration, and every strict Sonar criterion.
- Observability updates:
  - Exhaustion atomically labels each expected response, reports absent responses explicitly, and marks truncated evidence with shown and total byte counts.
- Status-doc validation:
  - Rechecked the ADR catalogue and documentation summary; this corrective CI behavior changes no product capability or operator workflow.
- Risk & rollback plan:
  - The primary risk is accepting or printing a stale mixed response set. Per-attempt deletion plus focused partial-fetch coverage constrains that risk.
  - Rollback is removal of this correction and its tests, though that would restore immediate transient-fetch termination, stale evidence retention, and incomplete diagnostics.
- Dependency rationale:
  - No dependency was added or changed; the implementation uses existing Bash and standard command-line tools.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/devops.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md` for task-record, shell-safety, Sonar strictness, and drift requirements.
  - No policy drift or contradiction was found, so no instruction file change is required.
