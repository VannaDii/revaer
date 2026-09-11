# Compliance manifest failure boundary

- Status: Accepted
- Date: 2026-09-11
- Operator approval: 2026-09-11, explicit C1/C1-D and conditional E1 approval
  in [ADR 588](588-first-release-decision-package.md#approval-resolution).
- Implementation status: Approved for implementation; not package-qualified.
- Context:
  - Provider reconstruction reaches a retained loader that converts filesystem
    and JSON failures into `None`, then reports a digest-shaped unavailable
    sentinel. Root policy prohibits hiding recoverable failure in absence,
    sentinel values or logs. G1 reserves observable failure classification and
    availability changes for operator approval.
- Decision:
  - C1 below is accepted together with ADR 588's exact C1-D delivery contract.
    No degraded fallback or criteria exception is selected.
- Consequences:
  - The recommendation makes the packaged manifest a startup prerequisite.
    Missing or malformed evidence stops startup rather than degrading silently.
  - Development and E2E bootstrap must supply real valid fixture evidence or
    explicitly inject a test loader; they may not manufacture a production
    digest or treat a missing artifact as verified compliance.
- Follow-up:
  - Implement and qualify the approved loader/bootstrap/delivery contract.
    Earlier pending wording below is retained proposal history; the exact
    ADR 588 resolution governs current implementation authority.

## Verified Conflict

At donor `d540f67c` and retained integration `65dbefb7`,
`crates/revaer-api/src/http/router.rs::load_source_compliance_bundle_digest`
reads `/app/compliance/final-image-compliance-bundle.json`. Read errors, JSON
errors, missing `source_compliance_sha256`, and invalid digest text are logged
and returned as `None`. A syntactically valid 64-character hexadecimal value
is returned with a `sha256:` prefix. This is a format check, not an independent
content, authenticity or package verification.

`with_config_with_media` places that optional result into API state. The media
compliance handler replaces absence with
`unavailable-until-final-image-bundle-is-present` and can return HTTP 200. The
same absence representation therefore covers several real failures. Retaining
logs does not satisfy the root's explicit `Result` requirement.

The newly validated schema-only children do not introduce this loader. The
conflict arises at the next provider/bootstrap owner; it must not be silently
reintroduced or changed under a mechanical-reconstruction claim. The existing
operator approvals for R1-R4, B1-B3, D1/D2/conditional D3, S1 and narrow F1 do
not select this startup failure policy.

## C1: Recommended Startup Failure Policy

Approve the following bounded behavior together:

1. Load the existing manifest and its existing digest field through an injected
   bootstrap collaborator before spawning application background tasks or
   accepting requests. Preserve the existing production location and digest
   syntax; do not add a new environment override, bypass or numeric limit.
2. Return an explicit typed error for unreadable/missing files, malformed JSON,
   absent/non-string digest fields and invalid digest text. Log once at the
   origin with a bounded cause category, without document contents or invented
   evidence. Propagate to a failed startup and nonzero process exit.
3. Inject the successfully loaded digest into the real media-enabled API state.
   Remove the failure-to-absence conversion and unavailable digest sentinel
   from this production path. Keep successful response fields and values
   unchanged. Test-only collaborators must remain explicit and must not be
   selected by production fallback logic.
4. Do not add a degraded-ready mode, HTTP status alternative, retry loop,
   partial media admission, shutdown deadline or process-containment fallback.
   Performing this check before task creation avoids selecting new cleanup
   semantics for tasks that have already started. S2 remains held.
5. Do not call a parsed digest verified package evidence. Actual manifest
   integrity, complete artifact closure, Linux amd64/arm64 package tests and
   all existing compliance gates remain separately required.

This would intentionally make all service startup unavailable when required
manifest metadata is absent or malformed, including otherwise unrelated API
features. That availability consequence is the reason approval is required.
It is not permission to weaken any GitHub, Sonar, dependency or release check.

## Alternatives

- Retain the current absence/sentinel path. This does not satisfy root policy;
  it would require an explicit separately scoped policy exception. None is
  requested or recommended here.
- Keep the process running with an explicit failed compliance state, HTTP
  errors, degraded readiness and media-admission fencing. This preserves
  unrelated service availability but requires a larger state/health/admission
  contract. It is not selected or partially implemented by C1.

## Validation Required After Approval

- Exercise real missing files, access failures, malformed JSON, absent/wrong
  field types, invalid hexadecimal/length and valid upper/lowercase digests.
  Assert typed errors, exact event counts and no source-content leakage.
- Prove failure occurs before any background-task spawn, listener admission or
  media command; prove the top-level process exits unsuccessfully. Do not use
  boolean-only mocks as a substitute for the real bootstrap failure path.
- Prove valid injected fixture evidence starts the actual application, exposes
  the unchanged successful compliance response and records real capabilities.
  Verify no production fallback selects fixture data.
- Run full `just ci`, `just ui-e2e`, authoritative Sonar and applicable GitHub
  checks, then validate real amd64/arm64 package manifests independently. None
  of these outcomes is established by this proposal.

## Task Record

- Motivation: Resolve a concrete policy conflict before exposing the first
  reconstructed real provider routes, without inventing operator consent.
- Design notes: C1 selects early failure rather than a new degraded-service
  state machine. It preserves successful wire shape and existing file/field
  names; private typed error layout remains an implementation detail.
- Test coverage summary: Read-only donor and integration inspection establishes
  the conversion path. No new runtime test, filesystem failure experiment or
  service behavior is claimed. Documentation validation is recorded in ADR581.
- Observability updates: Proposed bounded error classification only; no logger,
  severity, health, readiness or error response is changed in this task.
- Status-doc validation: Reviewed ADR559's G1 boundary, ADR564's completion
  ledger, root policy and retained provider code. Operator guides remain
  unchanged because C1 is not implemented. Both ADR indexes are updated.
- Risk & rollback plan: Missing package evidence would prevent startup under
  C1. Rejection keeps affected reconstruction unpublished pending another
  approved contract; it does not authorize restoring a hidden-failure fallback.
- Dependency rationale: No dependency is added or proposed. Existing standard
  file I/O, JSON parser and injected bootstrap patterns suffice.
- Stale-policy check: Reviewed root AGENTS and Rust/devops scoped guidance.
  Found retained loader behavior inconsistent with failure/absence policy;
  no policy is relaxed and no existing approval is rewritten. The fix remains
  pending a named decision rather than treating an agent delegation as consent.
