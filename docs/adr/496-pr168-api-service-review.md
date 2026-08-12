# PR 168 API service review remediation

- Status: Recorded
- Date: 2026-08-15
- Operator approval: Not applicable: nonarchitectural task record
- Context:
  - The replayed PR 168 API-service layer required an independent review against the accepted media architecture and service contracts.
  - The review found implementation drift in worker-attempt fencing, evidence-route exposure, unavailable-service errors, compliance artifact access, compatibility-target clearing, and desired-stream schema coverage.
- Decision:
  - Preserve the accepted worker-owned evidence model by removing the public phase-append route and its OpenAPI operation.
  - Carry the active claim generation through every worker evidence append parameter.
  - Report an absent media application service as HTTP 503 instead of returning synthetic configuration or diagnostic data.
  - Load and validate the source-compliance digest once in API bootstrap wiring, then inject it into `ApiState` for handler reads.
  - Preserve an explicitly empty compatibility-target patch as the stored-procedure clear sentinel while continuing to distinguish an omitted field.
  - Export the implemented audio loudness and dynamic-range fields in the desired-stream OpenAPI schema.
- Consequences:
  - Stale workers can be fenced by downstream stored-procedure adapters without losing the claim generation at the API facade boundary.
  - Public media evidence endpoints remain read-only and the API fails closed when the media service is not wired.
  - Compliance metadata no longer performs synchronous filesystem I/O or silently discards parsing failures in a request handler.
  - Profile patches can clear an optional compatibility target, and generated clients can represent every implemented desired-audio policy field.
- Follow-up:
  - Downstream media facade implementations must populate the added claim-generation fields when calling the fenced stored procedures.

## Task Record

- Motivation:
  - Close clear implementation defects in the immediate `refs/replay/media3/79..HEAD` API layer before stack integration.
- Design notes:
  - Changes implement ADR 318 and ADR 433 without introducing a new architectural decision.
  - Capability absence remains a truthful readiness state; other operations on the unwired facade return a typed unavailable error.
- Test coverage summary:
  - Expanded the route regression to cover every evidence collection, added unavailable-service mapping and clear-sentinel coverage, and asserted both audio policy fields in the desired-stream schema.
  - Updated handler tests to assert fail-closed HTTP 503 behavior and ran `just check`, `just lint`, and the complete isolated-database `just test` suite.
- Observability updates:
  - Bootstrap logs source-compliance bundle read, parse, missing-field, and invalid-digest failures once at their origin.
- Status-doc validation:
  - `README.md`, status documents, and operator guides were reviewed for impact; no status change was required.
- Risk & rollback plan:
  - Risk is limited to clients that incorrectly depended on a worker-only phase mutation route or synthetic responses from an unwired service.
  - Roll back this commit if integration reveals an accepted contract mismatch, then restore only behavior supported by an approved ADR.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Reviewed `AGENTS.md` and `.github/instructions/rust.instructions.md`.
  - No policy drift, contradiction, or stale reference was found.
