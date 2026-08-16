# Media sidecar discovery policy

- Status: Accepted
- Date: 2026-08-13
- Operator approval: Approved as written by the operator on 2026-08-15: "I approve the ADRs as they are now and I'm resuming your goal."
- Context:
  - Profiles persist ordered subtitle-discovery patterns and jobs snapshot those
    rules immutably.
  - The worker has no reader for the snapshot rows. Production inspection instead
    constructs a filesystem discoverer with three fixed filename forms and a
    fixed supported-extension set.
  - The persisted pattern text has length and ordering constraints but no defined
    grammar, compiler, escaping rules, complexity bound, or invalid-pattern
    behavior.
  - Pattern semantics affect filesystem traversal, resource use, matching
    determinism, security, and operator-visible configuration.
- Decision:
  - Select Option A, the restricted token grammar, as approved by the operator.
  - **Option A: restricted token grammar.** Support a versioned, anchored filename
    grammar using explicit tokens such as `{stem}`, `{lang}`, `{role}`, and
    `{ext}` with no path separators or recursive matching. Compile ordered enabled
    job-snapshot rules into a bounded matcher. This directly models the documented
    examples and keeps traversal adjacent to the source.
  - **Option B: bounded glob grammar.** Permit a reviewed glob subset with
    explicit traversal depth, entry, byte, and match limits. This is more flexible
    but expands ambiguity and filesystem reach.
  - **Option C: regular expressions.** Match adjacent filenames with a restricted
    regex engine and explicit complexity limits. This is expressive but exposes a
    configuration language broader than the documented product need.
  - Option A is deterministic,
    dependency-free, narrowly matches the v1 specification, and preserves the
    current adjacent-file trust boundary.
- Consequences:
  - The selected grammar becomes a versioned API, UI, import/export, validation,
    snapshot, worker, and audit contract.
  - Invalid or unsupported snapshotted rules must fail the job before probing
    candidate sidecars; they may not fall back to hard-coded defaults.
  - Resource limits remain independently enforced after pattern matching.
- Follow-up:
  - Define the grammar and stable validation errors, add stored-
    procedure-backed readers, and inject the compiled per-job discoverer through
    bootstrap/runtime composition.
  - Add fixture coverage for precedence, disabled rules, ambiguity, hostile names,
    paired VobSub files, and resource-budget exhaustion.

## Task Record

- Motivation:
  - Make the already persisted and snapshotted discovery policy meaningful before
    claiming v1 subtitle policy completeness.
- Design notes:
  - This ADR does not choose subtitle retention, conversion, OCR, or image-
    subtitle behavior.
  - Discovery remains fail-closed and limited to trusted source-root resolution.
- Test coverage summary:
  - Evidence gathered from the specification, profile and job snapshot schema,
    inspection adapter, and filesystem discoverer.
  - No implementation or behavioral test has been added yet.
- Observability updates:
  - A future implementation must record the rule version and precedence that
    matched each sidecar and expose stable invalid-rule and budget failures.
- Status-doc validation:
  - Rechecked the subtitle target and immutable snapshot sections of
    `MEDIA_TRANSCODING.md`; no existing document defines a broader grammar.
- Risk & rollback plan:
  - No runtime risk before approval.
  - After implementation, rollback must retain validation and must not interpret
    stored patterns under different semantics without a version transition.
- Dependency rationale:
  - No dependency proposed. Option A can use existing standard-library parsing.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and
    `.github/instructions/revaer-data.instructions.md`; no scoped rule defines
    discovery-pattern semantics.
