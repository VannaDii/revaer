# Canonical root identity paths

- Status: Recorded
- Date: 2026-09-10
- Operator approval: Not applicable: nonarchitectural task record
- Context:
  - The accepted root identity encoder promises normalized scalar validation
    and rejection of overlaps visible in canonical paths. Its lexical overlap
    comparison previously admitted empty, current-directory and parent-directory
    components, including `//same/child` beside `/same` with distinct claimed
    device/inode pairs.
  - These are unproved caller claims, not actual descriptor evidence. The
    defect does not establish a demonstrated filesystem escape or a working
    root-authority implementation.
- Decision:
  - Reject empty, `.` and `..` slash-separated components in the canonical
    claim before overlap comparison and identity framing. Retain the existing
    absolute, non-root, UTF-8 byte and NUL bounds.
  - Do not normalize input, consult the filesystem, change deployment-source
    parsing, or reject valid Unix filename bytes such as backslash, spaces,
    hidden names and Unicode. Accepted ADRs 523/550/557 remain the authority;
    this implements the existing normalized-canonical claim contract.
- Consequences:
  - Noncanonical aliases cannot produce identity frames through this boundary.
    Valid canonical inputs retain their exact bytes and golden digests.
  - Descriptor identity, mount aliases, capability probes, root locks and
    normalized database reconciliation are still separate required proofs.
- Follow-up:
  - Complete the approved trusted-root and logical-profile workflow. The
    existing UI/API automation failure is not fixed by identity validation;
    do not disable its safety gate or claim usable transcoding from unit tests.

## Task Record

- Motivation:
  - Close a concrete input-validation defect on the trusted-root critical path.
- Design notes:
  - Add the check to the existing scalar validator, before its pairwise overlap
    pass. No new type, public endpoint, filesystem authority or dependency.
  - Preserve the distinction between claimed identity encoding and attestation.
- Test coverage summary:
  - The new malformed-path regression fails against the prior validator.
    Nine malformed forms are checked in either slot position and seven in a
    single-slot catalog. Valid Unix filename cases assert the exact canonical
    field length/bytes and distinct identities, including composed/decomposed
    Unicode. A separate test preserves noncanonical requested-path bytes.
  - All 59 root-catalog tests pass with warnings treated as errors. Independent
    review confirmed the contract boundary and prompted the stronger cases.
  - At `e70ae058`, full `just ci` exits zero in the retained integration
    worktree: all-feature/minimal tests, strict lint, audits, all package
    coverage thresholds, script coverage and release build complete. Eight
    existing S2 configuration-watcher shutdown WARN events remain; this is not
    warning-free acceptance or approval of a shutdown bound.
  - Full `just ui-e2e` exits one: 46 pass, one fails and 61 do not run.
    `tests/specs/api/media.spec.ts:130` still receives 400 instead of 201 for
    profile creation with automation enabled before trusted root binding.
    Teardown rejects absent job-phase and profile-readiness route coverage.
    No assertion, automatic behavior or coverage criterion was removed.
  - Fresh Rust LCOV retains 238 source records, 101,426 line records and
    94,333 covered lines, alongside native and script coverage. No
    authoritative Sonar scan or published coverage metric was obtained.
  - Strict package Clippy, formatting, instruction drift, whitespace and
    documentation links pass (1,128 OK, zero errors). The correction is
    202/9,999 changed lines under the canonical no-rename guard.
- Observability updates:
  - Reuse `InvalidCanonicalPath`. No new log, metric, secret or path exposure.
- Status-doc validation:
  - ADR indexes are updated. Existing operator documentation and the completion
    ledger remain explicit that root attestation and bindings are incomplete.
- Risk & rollback plan:
  - A caller incorrectly labeling a noncanonical path as canonical now fails
    closed. Correct that caller's descriptor-derived evidence rather than
    weakening validation. Reverting this patch restores the demonstrated gap.
- Dependency rationale:
  - Standard string iteration only; no dependency or lockfile changes.
- Stale-policy check:
  - Reviewed root AGENTS, Rust scoped instructions, ADRs 523/550/557/559 and the
    root identity encoder contract. No rule change or new architecture approval
    is inferred; D4, D5, S2 and the package evidence holds remain unchanged.

## Evidence And Delivery Boundary

- Private red/green, full-gate and coverage evidence is retained under
  `/private/tmp/revaer-root-canonical-evidence-20260910`.
- This correction remains local pending reconciled stack delivery. No GitHub
  source, PR metadata, required check, merge or server-side criterion changed.
- The primary checkout's changes and conflicts are preserved. Completed
  temporary worktrees, the dedicated test database/storage and test media are
  removed after validation; committed refs and non-media evidence remain.
