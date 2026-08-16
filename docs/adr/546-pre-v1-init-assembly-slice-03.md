# Pre-v1 init assembly slice 03

- Status: Recorded
- Date: 2026-08-16
- Operator approval: Not applicable: bounded implementation of accepted ADRs 522 and 528
- Context:
  - ADR 522 requires the complete final-state init candidate to be assembled in
    statement-aligned pull requests that remain below the 8,500 changed-line
    assembly limit.
  - ADR 528 froze the 167-file migration corpus and proved a complete candidate
    with SHA-256
    `fb0d371f1afe365a72bbe34e9c8fe628e318be6de0e1f43b6f73c37897c95858`
    and 1,624 SQL statements.
  - ADR 545 established the second inert candidate prefix through statement 363,
    byte 528,189, and line 14,764.
  - Existing migrations remain the sole bootstrap authority until the later
    parity and cutover step accepted by ADRs 522 and 541.
- Decision:
  - Extend `crates/revaer-data/init.sql` to the exact first 782,363 bytes of the
    pinned candidate.
  - End this slice at candidate statement 485 and line 22,088. The tracked
    prefix SHA-256 is
    `a0eb1596814cefca9b4dbb78d1abdc16b8551aac30998163c82630fcd8460214`.
  - Preserve all 528,189 bytes from ADR 545 without modification and append
    only the next exact candidate bytes ending at the selected statement
    boundary.
  - Keep the migration directory, SQLx migration runner, application bootstrap,
    local recipes, and test setup unchanged. The partial init file remains
    review evidence only and is not executable by ordinary runtime or test
    paths.
  - Generate the candidate independently in this worktree before materializing
    the prefix. No candidate or database dump is committed.
- Consequences:
  - Reviewers can inspect the third bounded final-state SQL section while every
    existing bootstrap path continues to use the proven migration corpus.
  - PR CI regenerates the full candidate, verifies this exact prefix, and
    applies the prefix to an empty pinned PostgreSQL database.
  - Subsequent assembly slices may only append exact candidate bytes ending at
    another recorded statement boundary.
- Follow-up:
  - Append candidate bytes after byte 782,363 through the next selected
    statement boundary in the immediately following stack layer.
  - Do not select `init.sql`, delete migrations, or remove transition tooling
    until the complete candidate and two-way parity proof are present.

## Task Record

- Motivation:
  - Continue the accepted single-init transition without exceeding review
    limits or exposing an incomplete bootstrap authority.
- Design notes:
  - The SQL bytes are a mechanical prefix of the locally regenerated candidate;
    no SQL was hand-edited, reordered, normalized, or reinterpreted.
  - Statement boundary 485 adds 7,324 SQL lines and leaves review budget for
    this record and generated indexes.
- Test coverage summary:
  - `just db-init-prefix-check` accepted the 782,363-byte file as an exact,
    complete candidate prefix. The immediate-base diff appends 7,324 lines to
    `init.sql` without deleting or changing any byte from ADR 545's prefix.
  - `just db-rebaseline-candidate` regenerated the pinned 1,624-statement
    candidate, reproduced its digest, applied the complete candidate and this
    prefix to separate empty databases, and removed its disposable container.
  - `just policy`, `just check`, the complete `just test`, `just fmt`, and
    `just instruction-drift` passed. The documentation build passed through the
    pinned Cargo tool path while the separately tracked exact-tool path fix is
    pending integration; the link gate validated all 953 links without an
    error, and `git diff --check` reported no whitespace error.
  - Both canonical changed-line recipes passed against signed immediate-base
    commit `776cc905a485527c11403d918937fa35ea3f12fa`: the assembly gate measured
    7,441 changed lines against its 8,500 limit, and the stack gate measured
    the same 7,441 changed lines against its 9,999 limit.
- Observability updates:
  - Ignored local evidence retains the candidate digest, schema digests, prefix
    boundary map, and successful fresh-apply results without credentials or
    database contents.
- Status-doc validation:
  - Added this task record to the ADR index and mdBook summary. Runtime and
    operator bootstrap documentation remains migration-based because cutover
    has not occurred.
- Risk & rollback plan:
  - Revert this slice to restore the inert ADR 545 prefix. No database or
    application rollback is required because no bootstrap caller uses the
    partial file.
- Dependency rationale:
  - No dependency was added. Assembly uses the exact PostgreSQL image, SQLx CLI,
    Ruby guard, and SHA-256 tooling recorded by ADR 528.
- Stale-policy check:
  - Reviewed `AGENTS.md`,
    `.github/instructions/revaer-data.instructions.md`,
    `.github/instructions/devops.instructions.md`, ADR 522, ADR 528, ADR 541,
    ADR 545, the canonical database recipes, and PR workflow.
  - No stale reference or contradiction was found. No runtime bootstrap,
    required check, Sonar criterion, or review-size limit was relaxed.
