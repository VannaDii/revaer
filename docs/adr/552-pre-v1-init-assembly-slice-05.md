# Pre-v1 init assembly slice 05

- Status: Recorded
- Date: 2026-08-16
- Operator approval: Not applicable: bounded implementation of accepted ADRs 522, 528, and 541
- Context:
  - ADR 522 requires the complete final-state init candidate to be assembled in
    statement-aligned pull requests that remain below the 8,500 changed-line
    assembly limit.
  - ADR 528 froze the 167-file migration corpus and proved a complete candidate
    with SHA-256
    `fb0d371f1afe365a72bbe34e9c8fe628e318be6de0e1f43b6f73c37897c95858`
    and 1,624 SQL statements.
  - ADR 547 established the fourth inert candidate prefix through statement
    534, byte 1,034,699, and line 29,489.
  - ADR 541 keeps the migration corpus authoritative until the complete init
    script passes parity and the accepted packaged-bootstrap cutover is made.
- Decision:
  - Extend `crates/revaer-data/init.sql` to the exact first 1,306,068 bytes of
    the pinned candidate.
  - End this slice at candidate statement 781 and line 36,988. The tracked
    prefix SHA-256 is
    `d33a622322c4c09269614261c322c699643d4fd463172d8d7f89b4acb3dffed8`.
  - Preserve all 1,034,699 bytes from ADR 547 without modification and append
    only the next exact candidate bytes ending at the selected statement
    boundary.
  - Keep the migration directory, SQLx migration runner, application bootstrap,
    local recipes, and test setup unchanged. The partial init file remains
    review evidence only and is not executable by ordinary runtime or test
    paths.
  - Generate the candidate independently in this worktree before materializing
    the prefix. No candidate or database dump is committed.
- Consequences:
  - Reviewers can inspect the fifth bounded final-state SQL section while every
    existing bootstrap path continues to use the proven migration corpus.
  - PR CI regenerates the full candidate, verifies this exact prefix, and
    applies the prefix to an empty pinned PostgreSQL database.
  - Subsequent assembly slices may only append exact candidate bytes ending at
    another recorded statement boundary.
- Follow-up:
  - Append candidate bytes after byte 1,306,068 through the next selected
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
  - Statement boundary 781 adds 7,499 SQL lines and leaves review budget for
    this record and generated indexes.
- Test coverage summary:
  - `just db-init-prefix-check` accepted the 1,306,068-byte file as an exact,
    complete candidate prefix. The immediate-base diff appends 7,499 lines to
    `init.sql` without deleting or changing any byte from ADR 547's prefix.
  - `just db-rebaseline-candidate` independently regenerated the pinned
    1,624-statement candidate, reproduced its digest, applied the complete
    candidate and this prefix to separate empty databases, matched its
    normalized schema re-dump, and removed its disposable container.
  - `just policy`, `just instruction-drift`, `just docs`, and
    `just docs-link-check` passed. The link gate checked 967 links with zero
    errors, `git diff --check` reported no whitespace error, and `just ci`
    passed the complete repository gate.
  - `just ui-e2e` completed dependency audit, generated-client, browser,
    database-provisioning, application-startup, and JavaScript-coverage setup,
    then reproduced the inherited media API failure recorded by ADR 542. The
    unchanged test requested a legacy profile with `schedule_enabled=true`;
    migration 0182 returned HTTP 400 with
    `media_profile_filesystem_identity_required` and SQLSTATE `P0001` instead
    of the expected 201. Teardown consequently reported the profile-readiness
    and job-phase read routes uncovered. No test, coverage requirement, or
    failure was weakened or skipped.
  - The final immediate-base and stack diffs contain 7,622 additions and 2
    deletions (7,624 changed lines), below their respective 8,500 and 9,999
    limits.
- Observability updates:
  - Ignored local evidence retains the candidate digest, schema digests, prefix
    boundary map, and successful fresh-apply results without credentials or
    database contents.
- Status-doc validation:
  - Added this task record to the ADR index and mdBook summary. Runtime and
    operator bootstrap documentation remains migration-based because cutover
    has not occurred.
- Risk & rollback plan:
  - Revert this slice to restore the inert ADR 547 prefix. No database or
    application rollback is required because no bootstrap caller uses the
    partial file.
- Dependency rationale:
  - No dependency was added. Assembly uses the exact PostgreSQL image, SQLx CLI,
    Ruby guard, and SHA-256 tooling recorded by ADR 528.
- Stale-policy check:
  - Reviewed `AGENTS.md`,
    `.github/instructions/revaer-data.instructions.md`,
    `.github/instructions/devops.instructions.md`, ADR 522, ADR 528, ADR 541,
    ADR 547, the canonical database recipes, and PR workflow.
  - No stale reference or contradiction was found. No runtime bootstrap,
    required check, Sonar criterion, or review-size limit was relaxed.
