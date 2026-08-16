# Root catalog source implementation

- Status: Recorded
- Date: 2026-08-16
- Operator approval: Implementation of accepted ADR 550 was explicitly directed
  by the operator on 2026-08-16.
- Context:
  - Accepted ADRs 523 and 550 make an injected startup source the sole authority
    that may introduce absolute media-root path bytes.
  - The approved first slice is limited to the version-1 typed model, exact JSON
    validation, canonical semantic digest, source trait, and trusted local file
    source in `revaer-media-runtime`.
  - Database persistence, API and UI surfaces, profile binding, root evidence
    attestation, package manifests, and destructive readiness remain separate
    implementation slices.
- Decision:
  - Parse only the exact version-1 top-level and slot field sets, reject unknown
    and duplicate fields or enum values, enforce all decoded and raw bounds, and
    validate the accepted durability and sole-writer declaration matrices.
  - Normalize only slot and kind order. Preserve decoded key and path bytes and
    compute SHA-256 over ADR 550's exact domain-separated canonical frame.
  - Expose an injected `RootCatalogSource` that distinguishes a trusted loaded
    document from a missing-document remediation state. A missing source returns
    the canonical empty catalog and never implies `/data` or another root.
  - Open local sources component-by-component from the root directory descriptor
    with no-follow and close-on-exec semantics. Reject noncanonical source
    locations rather than silently normalizing them.
  - Require trusted ancestry ownership and non-group/world-writable modes,
    exact final-file ownership, a regular file with one hard link, and stable
    descriptor/path identity and metadata before and after parsing.
  - For packaged sources, require a non-root service identity, root-owned
    ancestry and file, failed effective write probes for every directory and
    the file, and the fixed `/etc/revaer/media-root-catalog.json` location.
    Native overrides require the effective operator identity and operator-owned
    file.
  - Read at most 8,388,609 bytes so one extra byte proves the 8,388,608-byte
    document bound. Public local loading remains unsupported outside Linux
    `amd64` and Linux `arm64`; cross-platform parser and descriptor-mechanism
    tests grant no attestation or readiness.
- Consequences:
  - Runtime consumers can receive one deterministic typed catalog and semantic
    digest without accepting raw paths from remote or persisted product state.
  - Symlink, hard-link, special-file, replacement, mutation, unsafe-owner, and
    unsafe-mode inputs fail closed with ADR 550's bounded reason classes.
  - The loaded catalog is still only deployment input. No slot is attested,
    bound, writable, or ready for destructive work in this slice.
- Follow-up:
  - Add package document generation and mounts only in the separately scoped
    Docker and Helm slice.
  - Add resolver, root attestation, persistence, binding, administration, and
    readiness behavior only in their approved slices.
  - Validate the packaged root-owned read-only mount on Linux `amd64` and
    `arm64`, including privileged mount replacement, when package manifests are
    implemented. The current macOS host can prove parser and Unix descriptor
    behavior but correctly reports the public source as unsupported.

## Task Record

- Motivation:
  - Supply the first bounded implementation needed to replace raw profile paths
    with accepted ADR 523 logical root authority.
- Design notes:
  - Validated public model fields are private and can only be constructed by the
    exact parser. Public APIs expose inspection getters and no alternate
    deserialization or mutation path.
  - Kind masks and all four evidence enum bytes use ADR 550's exact assignments;
    independently fixed digest vectors protect the canonical frame from a
    self-confirming implementation test.
  - Source paths are retained byte-for-byte. Dot, parent, repeated-separator,
    and trailing-separator components are rejected for trusted source locations
    because accepting them would silently normalize untrusted authority.
  - Directory and final-file descriptors are re-opened and compared after the
    bounded read and after typed parsing. File comparisons include device,
    inode, mode, link count, owner, group, size, modification time, and change
    time; directory comparisons retain identity, owner, group, and mode.
  - Files are opened nonblocking after no-follow type inspection so FIFO and
    other special-file substitutions cannot stall bootstrap.
- Test coverage summary:
  - `just test-media-root-catalog` passes 37 focused tests with zero failures.
  - Parser tests cover empty, one-below, at, and one-above slot, key, decoded
    path, override path, and raw-document bounds; invalid UTF-8 and NUL; exact
    and unknown fields and enums; every duplicate field; duplicate keys and
    kinds; invalid class/evidence pairs; malformed versions and numbers;
    multiple or trailing JSON values; escape amplification; all five kind bits;
    ordering invariance; and independently fixed one-slot and empty digests.
  - Host filesystem tests cover trusted load, missing file and parent, symlinked
    final and parent entries, hard links, directories, Unix sockets, FIFOs,
    owner mismatch, group/world-writable files and ancestry, noncanonical
    locations, source-path redaction, exact one-extra-byte reading, and public
    unsupported-platform behavior.
  - Deterministic phase hooks prove same-length replacement at open, during
    read, after read, and after parse, plus truncation, extension, chmod, unlink,
    hard-link creation, final rename/replacement, and parent
    rename/replacement.
  - `just check`, the complete `just lint` policy, both strict Clippy profiles,
    and a fresh complete `just ci` run pass. The first `just ci` invocation hit
    one unrelated timing-sensitive failure in
    `process::tests::deadline_includes_pipe_setup_time`; a complete `just test`
    rerun and then the fresh complete `just ci` run both passed without a source
    change.
  - `just docs-index`, `just instruction-drift`, `just docs-build`, and
    `just docs-link-check` pass. Link validation checked 977 links, 499 unique,
    with zero errors.
  - The scoped security diff review closed all seven changed production files
    with complete coverage and zero reportable findings.
  - `just ui-e2e` was run and did not pass: 46 tests passed, one test failed,
    and 61 tests did not run after profile creation in
    `tests/specs/api/media.spec.ts` returned 400 instead of 201. Teardown then
    reported uncovered profile-readiness and job-phase routes. The failure is
    the existing `media_profile_filesystem_identity_required` contract; fixing
    it requires profile-to-root identity binding, which this approved slice
    explicitly excludes.
- Observability updates:
  - Missing sources expose only `media_root_catalog_source_missing`; errors map
    to `media_root_catalog_source_untrusted`,
    `media_root_catalog_format_invalid`,
    `media_root_catalog_bound_exceeded`, or
    `media_root_platform_unsupported`.
  - Error messages retain bounded operation and operating-system evidence but
    never include the configured source path. No logs, metrics, API fields, or
    database records are added.
- Status-doc validation:
  - Product capability status does not change because no bootstrap wiring,
    profile binding, root attestation, package artifact, or readiness path is
    activated.
  - The ADR index, mdBook summary, and generated documentation catalogs include
    this implementation record.
- Risk & rollback plan:
  - Primary risks are parser identity drift and filesystem replacement races.
    Fixed digest vectors, exact bound tests, no-follow descriptor traversal, and
    repeated metadata/path checks lock those behaviors.
  - Actual root-owned packaged mounts and privileged mount replacement cannot be
    proven on the current macOS host and remain explicit package-slice evidence
    requirements; they are not inferred from the host tests.
  - Rollback removes this isolated module, its focused recipe, and the direct
    `sha2` crate declaration. No persistence or externally visible contract
    requires data rollback.
- Dependency rationale:
  - `sha2.workspace = true` is added to `revaer-media-runtime` for the mandated
    semantic SHA-256 digest. The exact `sha2` 0.10.9 package was already pinned
    and used in the workspace. This adds the required direct crate edge to that
    existing package but no new package, version, or transitive dependency in
    the resolved workspace dependency set.
  - Existing `serde_json` performs structured parsing and duplicate/unknown
    field rejection. Existing `rustix` provides safe descriptor-relative,
    no-follow, metadata, identity, and effective-access operations. No other
    dependency is added.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/devops.instructions.md`, ADR 523, and accepted ADR
    550.
  - No policy contradiction or stale product claim was found. The devops
    instruction now names the added canonical focused `just` recipe; no lint,
    test, coverage, Sonar, platform, trust, or readiness criterion is relaxed.
