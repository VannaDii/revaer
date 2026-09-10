# Root attestation identity encoding

- Status: Recorded
- Date: 2026-09-10
- Operator approval: Not applicable: nonarchitectural task record
- Context:
  - Accepted ADR 557 needs exact identity bytes before normalized root binding
    can be implemented. Its approval does not establish descriptor evidence.
- Decision:
  - Implement the accepted ADR 557 slot, aggregate attestation, and generation
    framing with ADR 517 primitives and ADR 550's existing typed enums/SHA-256.
    This record introduces no architectural choice or changed contract byte.
  - An injected loaded catalog is mandatory. Explicit `RootSlotIdentityClaims`
    are validated against its complete declarations; the only output is
    `RootCatalogIdentityEncoding`, never an attestation or a readiness token.
- Consequences:
  - Pure encoding and normalized-field coherence are exercised independently
    of Linux. This does not establish a descriptor, mount, ownership, lock,
    deployment, capability-probe, package, or persistence proof.
  - Requested paths, kinds, and class/evidence pairs come from an exact matching
    parsed declaration. No caller digest, timestamp, database id, or generation
    occurrence can enter the encoding. An identical semantic generation digest
    must not be treated as reuse of an old numeric generation fence.
- Follow-up:
  - Parent integrates this prerequisite and owns full `just ci`, `just ui-e2e`,
    Sonar, and stack validation. Independent PostgreSQL digest verification,
    descriptor attestation, root binding, and activation remain separate work.

## Task Record

- Motivation:
  - Supply the complete bounded identity component for the operator-approved
    root contract without fabricating the attestation needed by root binding.
- Design notes:
  - At most 256 claims must match the loaded source exactly once. Encoding is
    ordered by decoded logical-key bytes. Missing sources are distinct from a
    successfully loaded empty document. Existing parsing owns key/path bounds,
    kind order, closed enum mappings, and source/output declaration coherence.
  - Claimed canonical paths and filesystem types respect the normalized text
    bounds and SQL NUL prohibition. Device/inode are unsigned u64; mount ids fit
    nonnegative SQL bigint; UID/GID use the complete u32 range; mode is 0..4095.
  - Seven booleans produce exactly the seven capability bits. Source needs read;
    writing kinds need all six write-related capabilities and declared exclusive
    control. Disposable claims do not become destructive-ready by encoding.
  - Claimed canonical equality/ancestry and duplicate device/inode pairs fail.
    Sharing a mount id alone does not fail. Descriptor/mount-table alias proof is
    explicitly outside this pure component, not replaced by string checks.
- Test coverage summary:
  - Fixed slot/aggregate/generation and empty-document golden vectors were
    transcribed independently from ADRs 517/550/557; Ruby OpenSSL SHA-256 hashed
    the literal frames, not bytes emitted by the Rust encoder. Fixtures include
    UTF-8 byte lengths, unsigned extremes, enum bytes 0/1/2, and capability 126
    for a legitimate non-reading workspace declaration.
  - Additional tests exercise all 31 nonempty kind masks, all 128 possible
    seven-bit capability masks, all accepted class/evidence pairs, mutation and
    order invariants, source metadata exclusion, scalar/slot bounds, mismatched
    declarations, missing/extra/duplicate slots, and observable overlap.
  - An exhaustive input-schema destructuring test prevents timestamp or database
    occurrence columns from silently entering the input type; semantic encodings
    are repeatable independently of source metadata.
  - `just fmt`, `just test-media-root-catalog` (55 tests, including 16 identity
    tests), `just lint` (policy plus both strict workspace Clippy passes), and
    `just instruction-drift` passed on macOS arm64. No warnings or errors remain
    in those final gates. Three initial test-layout assertions and three test
    lint findings were corrected without changing the approved golden vectors.
  - `just docs-index` generated 507 entries. `just docs-link-check` passed 1,046
    links. `just docs-build` succeeded with the existing large-search-index
    warning; this is not a warning-free documentation completion claim.
  - Full CI/UI, published coverage/Sonar, Linux packages, and PostgreSQL parity
    were not run for this isolated slice; parent integration retains those gates.
- Observability updates:
  - No runtime logging, metric, health, or readiness surface changes. Error
    variants carry no path, key, or filesystem identifier. Frames contain paths
    and are documented as unsuitable for general logs.
- Status-doc validation:
  - Reviewed ADR 557's full field/digest/generation contract, ADR 517 framing,
    ADR 550 encoding, and ADR 564's incomplete release evidence ledger. No
    operator workflow, database cutover, package, or service completion claim.
- Risk & rollback plan:
  - Main risk is framing drift. Independent literal vectors and mutation tests
    guard byte layout; PostgreSQL must still verify it independently at cutover.
    Revert this isolated component before integration if it regresses; no schema,
    stored procedure, bootstrap path, deployment, or persisted state is changed.
- Dependency rationale:
  - Reuse existing `sha2`, `thiserror`, parsed catalog enums, and standard
    collections. No dependency or recipe change.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and the
    relevant focused-recipe guidance in devops instructions. No criteria,
    accepted constants, approvals, or instruction contradictions changed.
