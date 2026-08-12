# Sonar API Verification Retries

- Status: Accepted
- Date: 2026-08-11
- Context:
  - The post-scan verifier is a required merge gate and must distinguish a transient Sonar API failure from invalid or noncompliant analysis evidence.
  - The verifier previously delegated HTTP failure handling to `curl --fail`; this did not retain enough response detail to apply a narrow retry policy.
- Decision:
  - Capture and validate the HTTP status for every Sonar API request before accepting its response body.
  - Retry only transport failures, HTTP 429 rate limits, and HTTP 5xx server errors, using positive bounded attempts and a non-negative delay.
  - Treat all other HTTP failures and every malformed or noncompliant result as terminal.
  - Keep the existing positive coverage, lines-to-cover, quality-gate, issue, and hotspot assertions unchanged.
  - Alternatives considered:
    - Retry every non-success response: rejected because authentication and authorization failures require intervention rather than repeated requests.
    - Remove the post-scan API verifier: rejected because scanner completion alone does not prove that required metrics and review gates were published.
- Consequences:
  - Short Sonar outages and rate limits no longer create an immediate false-negative gate result.
  - Persistent service failures remain bounded and fail closed with the response body and status available for diagnosis.
- Follow-up:
  - Retain the regression fixture in `just policy` and review retry defaults if Sonar publishes different operational guidance.

## Task Record

- Motivation:
  - Restore the strict policy suite after the HTTP-aware verifier exposed an incomplete curl test double, while preserving maximal analysis enforcement.
- Design notes:
  - Retry controls are environment inputs validated before any request. Defaults remain five attempts with a three-second delay.
  - The test double emits explicit HTTP codes and persists per-endpoint call counts so retry behavior is exercised rather than inferred.
- Test coverage summary:
  - The guardrail regression suite covers successful evidence, PR scoping, zero coverage, ignored quality-gate conditions, unresolved issues, unreviewed hotspots, terminal authorization failure, transient 503 recovery, 429 recovery, missing credentials, and invalid retry controls.
- Observability updates:
  - Retry diagnostics identify the endpoint, HTTP status, current attempt, and maximum attempts. Terminal diagnostics retain a nonempty response body when available.
- Status-doc validation:
  - Product status and operator guides are unaffected. ADR indexes and Sonar-specific agent instructions were updated.
- Risk & rollback plan:
  - A badly chosen retry bound could delay a failed job. Inputs are validated and defaults cap that delay; rollback this ADR and the retry loop together if the policy causes operational regressions.
- Dependency rationale:
  - No dependency was added; the implementation uses Bash, curl, and jq already required by the gate.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/devops.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md`.
  - Drift was found in the Sonar-specific instructions, which did not state the post-scan API retry boundary. The instruction now requires bounded transient-only retries and terminal handling for criteria failures.
