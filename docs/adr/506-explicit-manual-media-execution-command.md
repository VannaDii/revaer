# Explicit manual media execution command

- Status: Accepted
- Date: 2026-08-16
- Operator approval: Explicitly approved by the operator on 2026-08-16:
  "I approve the ADRs as they are now and I'm resuming your goal."

## Context

- The source-stack ADR 362 (`362-media-worker-owned-job-creation.md`) removed
  `POST /v1/media/jobs` because it accepted a caller-authored job record that
  could bypass discovery's stable aggregate fingerprint and worker-owned state.
  It concluded that public clients must use discovery or automation.
- The current product contract in `MEDIA_TRANSCODING.md` lines 214-222 makes a
  different operator promise: manual discovery over a dry-run profile remains
  plan/audit-only, but an explicit manual job execution may override profile
  dry-run for that run when the operator supplies the exact phrase `replace`.
  The override never mutates the saved profile.
- Removing raw record creation was correct, but treating every explicit manual
  execution command as caller-owned persistence erased the required operator
  workflow. Command intent and durable job evidence are separate ownership
  boundaries.
- A manual execution must use the same descriptor-bound container-and-sidecar
  aggregate fingerprint, stored-procedure admission, immutable job snapshot,
  and worker-owned attempts and evidence as discovery-created work.

## Options

1. **Keep ADR 362 unchanged.** Remove explicit manual execution from the product
   contract and require discovery for every job. This preserves the smallest API
   but contradicts the resolved dry-run override workflow.
2. **Restore raw `POST /v1/media/jobs`.** Let callers submit job fields and
   evidence directly. This restores a route but recreates the ownership and
   fingerprint bypass that ADR 362 correctly rejected.
3. **Expose a manual execution command.** Accept only operator intent, derive all
   evidence and durable state on the server, and admit the resulting job through
   the same stored-procedure and fingerprint boundary as discovery.

## Recommendation

- Adopt option 3 and preserve ADR 362's rejection of caller-authored job state.
- Expose `POST /v1/media/profiles/{media_profile_public_id}/executions` as a
  command, not a record-creation endpoint. Its request accepts only the source
  path and an optional `dry_run_override_confirmation`.
- Dry-run behavior is exact:
  - When the saved profile is dry-run and the confirmation is absent, the command
    creates only plan and audit work.
  - When the saved profile is dry-run, destructive execution is requested only
    by the byte-for-byte ASCII value `replace`. The server performs no trimming,
    case folding, Unicode normalization, aliases, or boolean substitute.
  - Any other supplied confirmation is rejected with a stable invalid-request
    code.
  - When the saved profile is non-dry-run, no confirmation is required and a
    supplied override confirmation is rejected as inapplicable rather than
    silently ignored.
  - The effective one-run mode is persisted on the immutable job snapshot and
    never changes the saved profile.
- Manual discovery remains a separate preview/run workflow and never accepts the
  confirmation field or produces destructive work from a dry-run profile.
- Before admission, the service authorizes the command, resolves the profile and
  managed source root, performs descriptor-relative no-follow discovery and
  stability checks, deterministically enrolls owned sidecars, and computes the
  same stable aggregate fingerprint used by discovery.
- The server passes only validated intent plus server-derived profile, policy,
  aggregate fingerprint, and effective dry-run facts to a stored procedure. The
  procedure atomically claims the durable fingerprint and creates the immutable
  job snapshot or returns the existing bounded admission outcome.
- The request cannot provide a job identifier, fingerprint, capability or policy
  snapshot, attempt, claim generation, status, phase, operation, violation,
  reason, check, artifact, audit, timestamp, or worker evidence. Those remain
  server-derived and worker-owned.
- Queue admission does not authorize execution. The claimed job still passes the
  approved capability snapshot, source-bound preflight, command construction,
  cancellation, fingerprint revalidation, and replacement boundaries before any
  mutation.

## Consequences

- Operators retain the explicit one-run execution required by the product
  contract without regaining a general-purpose job-state mutation API.
- Direct commands and discovery converge on one source identity, deduplication,
  admission, and worker evidence model.
- Clients must treat the response as command acceptance or an existing admission
  result, not confirmation that execution succeeded.
- The service performs stability and aggregate fingerprint work before durable
  admission, so the endpoint must use existing scan budgets, cancellation, and
  bounded error handling.
- API models, OpenAPI, stored procedures, facade boundaries, authorization tests,
  dry-run tests, and direct/discovery fingerprint equivalence tests must change
  together during implementation.

## Implementation Boundary

- This accepted ADR supersedes only ADR 362's conclusion that explicit manual
  execution must be removed. It preserves ADR 362's rejection of raw
  caller-authored job records and evidence.
- This ADR authorizes only the profile-scoped command, exact `replace`
  semantics, server-derived aggregate fingerprint, stored-procedure admission,
  immutable effective dry-run fact, and rejection of caller-authored state
  described above.
- It does not authorize direct evidence append, arbitrary job fields, caller
  command fragments, unmanaged paths, discovery dry-run overrides, mutation
  before authoritative preflight, or bypasses of worker ownership, cancellation,
  capability, fingerprint, workspace, and replacement controls.
- Schema, runtime, API, and generated-contract changes must remain limited to the
  accepted contract above.

## Follow-up

- Implement routes, DTOs, services, stored procedures, schema, OpenAPI, generated
  clients, and UI only within the accepted boundary above.
- Add contract tests for absent, exact, near-match, differently
  cased, padded, and Unicode-lookalike confirmations on both dry-run and
  non-dry-run profiles.
- Prove direct and discovery admission compute the same aggregate identity for
  the same canonical container and sidecars, reject unstable or ambiguous
  aggregates, deduplicate atomically, and leave all phase and evidence writes to
  the claimed worker attempt.

## Task Record

- Motivation:
  - Resolve the PR 108-120 replay conflict between ADR 362 and the explicit
    manual execution behavior already specified in `MEDIA_TRANSCODING.md`.
- Design notes:
  - A profile-scoped verb names operator intent without reviving a generic job
    CRUD surface. The server converts that intent into the same immutable,
    fingerprinted aggregate admission used by discovery.
  - Exact confirmation is deliberately not normalized so automation cannot turn
    an approximate value into destructive authorization.
- Test coverage summary:
  - The ADR-only change added no runtime, API, or schema behavior.
  - `just policy`, `just instruction-drift`, `just docs-build`,
    `just docs-link-check`, and `git diff --check` passed.
  - The confirmation, fingerprint, admission, and ownership matrix in Follow-up
    remains mandatory before an accepted implementation can be handed off.
- Observability updates:
  - No telemetry changes are made by this ADR. A future implementation
    should distinguish manual command admission outcomes with stable bounded
    outcome codes and must not log the confirmation value or use profile,
    source, fingerprint, or job identifiers as metric labels.
- Status-doc validation:
  - Reviewed source ADRs 353, 355, 356, and 362; current ADRs 430, 442, 449, and
    500; and `MEDIA_TRANSCODING.md` lines 209-222. The recommendation preserves
    the documented manual override while making no claim that it is implemented.
  - `README.md`, roadmap/status documents, and operator guides were not changed;
    none currently claims this accepted contract is implemented.
- Risk & rollback plan:
  - Acceptance alone changes no production behavior. Any reversal requires a
    superseding ADR.
  - After implementation, disabling the command route is the simplest safe
    rollback. Existing admitted jobs must remain immutable and continue through
    normal worker cancellation or recovery; no caller-authored state path may be
    restored.
- Dependency rationale:
  - No new dependency is required. Existing descriptor-safe discovery, aggregate
    fingerprint, stored-procedure, authorization, and worker primitives are
    sufficient.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and
    `.github/instructions/revaer-data.instructions.md` as the prospective
    implementation constraints.
  - No instruction drift was found. ADR 362 conflicts with the current product
    contract only in its removal of explicit manual execution; this ADR
    preserves its state-ownership rationale and records the narrow correction.
