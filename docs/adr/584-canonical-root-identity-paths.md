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
    Full CI and
    UI E2E remain required and are recorded separately when observed.
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
