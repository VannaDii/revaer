# Immutable media capability execution identity

- Status: Accepted
- Date: 2026-08-16
- Operator approval: Explicitly approved by the operator on 2026-08-16:
  "I approve the ADRs as they are now and I'm resuming your goal."

## Problem

- Accepted ADR 449 binds a completed capability run on first successful claim,
  reuses it for retry and resume, and requires fail-closed executable identity
  validation. It does not define executable identity, binding storage, complete
  run retrieval, or host-upgrade behavior.
- A path and version string do not prove which bytes or dynamically loaded
  components will execute. A binary digest alone does not prove the library,
  plugin, driver, or packaged runtime closure that supplied detected codecs.
- Accepted ADR 501 requires one injected supervisor for every native media tool.
  Capability validation and process execution must therefore consume the same
  identity contract rather than race through separate path lookups.

## Options

1. **Bind normalized path and version output.** This is portable and cheap but
   permits byte or dependency replacement without changing the binding.
2. **Bind a versioned execution-closure identity.** Hash the executable and its
   approved runtime closure, bind one completed run immutably, and make the
   process supervisor validate the same closure before use.
3. **Bind only a container image digest.** This is strong for packaged
   deployment but excludes local development and supported non-container
   supervisors and does not identify mutable devices or drivers.

## Recommendation

- Adopt option 2, with the packaged image digest retained as additional evidence
  when available.
- A completed capability run has one `execution_environment_identity_v1` and one
  or more `native_tool_identity_v1` rows. The environment identity contains:
  - operating-system family, kernel ABI family, and CPU architecture;
  - packaged OCI image digest and package-manifest digest when packaged;
  - normalized native-library closure digest;
  - hardware device class, stable device identity, driver version, and runtime
    API version for every hardware capability advertised by the run;
  - identity-contract version and SHA-256 digest algorithm.
- Each native tool identity contains:
  - a closed logical role such as `ffmpeg`, `ffprobe`, `exiftool`, or
    `ccextractor`;
  - the configured executable slot and canonical absolute path used for audit;
  - file byte length and SHA-256 digest;
  - normalized executable format and architecture;
  - bounded normalized version output;
  - an ordered dependency-closure manifest of logical library or module name,
    canonical path, byte length, and SHA-256 digest;
  - a canonical digest over the complete tool row and dependency manifest.
- Scripted tools include the interpreter and every loaded shipped module in the
  closure. Plugin-capable tools include every plugin directory and module that
  can supply an advertised capability. An unresolved, writable, missing, or
  unbounded dependency makes the run incomplete.
- Canonical paths and dependency names are audit evidence but the execution-
  closure digest covers their normalized values as well as bytes. Relocation is
  therefore a new identity even when bytes match; it requires a new capability
  run and explicit re-plan rather than implicit equivalence.
- Capability probing runs through the same injected ADR 501 supervisor and the
  same opened executable identity that later execution will use. A supported
  production supervisor must prevent path substitution between validation and
  spawn, or prove the containing package tree is immutable to the service and
  unprivileged writers. A platform that can prove neither remains dry-run-only.

### Immutable Binding

- Store bindings as immutable rows keyed by `(media_job_id, plan_generation)`.
  The row records a public binding id, completed capability-run id, execution-
  environment digest, bound timestamp, first attempt number, first claim
  generation, and binding-contract version.
- The first successful claim for plan generation 1 calls one generation-fenced
  stored procedure that locks the job planning row, selects exactly one latest
  **completed and internally valid** run visible in that transaction, inserts
  the binding if absent, and returns the complete binding. A concurrent caller
  receives the same row or a stale-claim error; it cannot create a second row.
- Retry and resume read the existing binding by job and plan generation. They do
  not query the latest run. Accepted ADR 520 is the only path that may create a
  later plan generation with a different binding.
- A run-id reader returns the complete immutable environment, tool, codec,
  encoder, decoder, muxer, demuxer, subtitle, hardware, filesystem, and utility
  rows. Missing child rows, duplicate logical keys, unknown identity versions,
  or count/digest mismatch invalidates the run.

### Host Upgrade And Execution Validation

- Startup and on-demand refresh always create a new immutable run; they never
  edit a completed run or existing binding.
- Before planning, command materialization, checkpoint reuse, and every native
  spawn, the supervisor validates the currently opened execution closure against
  the bound tool and environment digests. It must also prove the required
  capability row belongs to that same run.
- Any executable, dependency, path, package, architecture, device, driver, or
  runtime mismatch fails before the native process starts with
  `media_capability_execution_identity_mismatch`. It does not fall back to the
  latest run, another encoder, software processing, or a version-string match.
- A host upgrade that makes the bound closure unavailable leaves the job
  operator-remediable and preserves all prior plan, attempt, and checkpoint
  evidence. The operator either restores the exact closure or invokes the
  explicit re-plan contract in ADR 520.
- Capability refresh failure does not fabricate a run and does not erase the
  last completed run. Accepted ADR 514 defines subsystem degradation; this ADR
  does not redefine its lifecycle model.

### Stable Errors

- Use stable bounded codes for `capability_run_missing`,
  `capability_run_incomplete`, `capability_run_identity_invalid`,
  `capability_binding_claim_not_current`, `capability_binding_conflict`,
  `capability_binding_missing`, `capability_tool_missing`,
  `capability_execution_identity_mismatch`, and
  `capability_requirement_not_in_bound_run`.
- Logs and API evidence may identify logical tool role, identity version, and
  mismatch category. They do not emit executable bytes, full host paths,
  command arguments, environment values, or dependency manifests by default.

## Consequences

- Plans and retries can identify the exact native execution closure that made a
  capability claim, including dependencies and hardware state.
- Capability refresh and host upgrade become explicit state transitions rather
  than silent changes to queued work.
- Closure discovery and hashing add startup and refresh cost and require
  platform-specific supervisor support. That cost is bounded and outside the
  per-file execution path after an identity is validated and held safely.
- Bare-metal production has a higher attestation burden than packaged execution;
  unsupported mutable installations remain non-destructive.

## Implementation Boundary

- This accepted ADR authorizes only identity fields, closure hashing, immutable
  run completeness, `(job, plan generation)` binding, first-claim atomicity,
  complete run reads, pre-use validation, host-upgrade failure, and stable errors
  described above.
- Accepted ADRs 449 and 501 remain authoritative for binding time and process
  supervision. ADR 520 separately owns re-plan authorization and evidence.
- This ADR does not authorize a new native tool, encoder fallback, capability
  fabrication, raw command logging, mutable completed runs, automatic re-plan,
  or unrelated behavior from ADRs 507-516. It preserves the exact-value holds in
  ADRs 515 and 516.
- Persistence changes belong in the pre-v1 `init.sql` under accepted ADR 522.
  Schema, runtime, package, API, workflow, and generated-contract changes must
  remain limited to the accepted contract above.

## Validation

- Decision validation to date is documentation-only.
- During implementation, add known-answer closure manifests for packaged and supported
  host execution, including scripted tools, dynamic libraries, plugins, and
  hardware identities.
- Test concurrent first claims, incomplete runs, duplicate children, path
  relocation, same version with changed bytes, same binary with changed library,
  driver change, device change, refresh failure, and exact restoration.
- Race-test executable replacement between validation and spawn and prove the
  supervisor either executes the validated object or fails before execution.
- Prove retry and resume always read the original binding and that only an
  authorized ADR 520 re-plan can select a newer run.
- An accepted implementation is not complete until packaged real-tool tests,
  focused concurrency tests, `just ci`, and `just ui-e2e` pass.

## Follow-up

- Define the supported packaged and bare-metal attestation mechanisms in the
  ADR 501 supervisor implementation plan before enabling destructive readiness.
- Implement accepted ADR 520 with this record so plan-generation storage and re-plan evidence
  use one schema.

## Task Record

- Motivation:
  - Complete executable identity and upgrade behavior deferred by accepted ADR
    449.
- Design notes:
  - Capability claims and process execution consume one execution-closure value;
    version text is evidence, not identity.
- Test coverage summary:
  - The ADR-only change added no capability, process, package, or database tests.
- Observability updates:
  - Future bounded metrics may use tool role and mismatch category only. Paths,
    digests, devices, jobs, and capability-run ids must not be labels.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, accepted ADRs 449, 484, 500, and 501, and
    accepted ADRs 507-516. Accepted lifecycle and re-plan behavior remains outside
    this ADR, and unresolved exact values remain held.
- Risk & rollback plan:
  - Any reversal requires a superseding ADR. A later rollback must preserve old
    closure and binding evidence and fail closed when unreadable.
- Dependency rationale:
  - No new dependency is required. Existing SHA-256, process supervision, package
    manifests, and platform inspection boundaries are sufficient to begin.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/ffi.instructions.md`, and
    `.github/instructions/devops.instructions.md`; no drift or relaxation was
    found.
