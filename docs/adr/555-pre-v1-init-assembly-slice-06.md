# Pre-v1 init assembly slice 06

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
  - ADR 552 established the fifth inert candidate prefix through statement 781,
    byte 1,306,068, and line 36,988.
  - ADR 541 keeps the migration corpus authoritative until the complete init
    script passes parity and the accepted packaged-bootstrap cutover is made.
- Decision:
  - Extend `crates/revaer-data/init.sql` to the complete 1,593,023-byte pinned
    candidate.
  - End this final assembly slice at candidate statement 1,624 and line 44,630.
    The completed candidate retains SHA-256
    `fb0d371f1afe365a72bbe34e9c8fe628e318be6de0e1f43b6f73c37897c95858`.
  - Preserve all 1,306,068 bytes from ADR 552 without modification and append
    only the remaining exact candidate bytes ending at the final statement
    boundary.
  - Keep the migration directory, SQLx migration runner, application bootstrap,
    local recipes, and test setup unchanged. The complete candidate remains
    review evidence only and is not executable by ordinary runtime or test
    paths.
  - Do not add the ADR 551 header, lifecycle procedures, grants, or final digest
    seal in this slice, and do not perform migration cutover or retirement.
  - Generate the candidate independently in this worktree before materializing
    the complete file. No candidate evidence or database dump is committed.
- Consequences:
  - Reviewers can inspect the final bounded SQL section while every existing
    bootstrap path continues to use the proven migration corpus.
  - PR CI regenerates the full candidate, verifies that `init.sql` is the exact
    complete candidate, and applies it to an empty pinned PostgreSQL database.
  - The accepted packaged-bootstrap finalization and cutover remain separate,
    reviewable work with no authority change hidden in assembly.
- Follow-up:
  - Implement and validate the accepted ADR 551 finalization in its own bounded
    stack layer.
  - Prove final two-way parity before selecting `init.sql`, deleting migrations,
    or removing transition tooling.

## Task Record

- Motivation:
  - Complete the accepted single-init candidate assembly without exceeding
    review limits or exposing a new bootstrap authority.
- Design notes:
  - The SQL bytes are a mechanical copy of the locally regenerated candidate;
    no SQL was hand-edited, reordered, normalized, or reinterpreted.
  - The final boundary adds 7,642 SQL lines and leaves review budget for this
    record and generated indexes.
- Test coverage summary:
  - `just db-init-prefix-check` accepted the 1,593,023-byte file as the exact,
    complete candidate. The original 1,306,068-byte ADR 552 prefix matched the
    regenerated candidate byte-for-byte before the final candidate was
    materialized.
  - `just db-rebaseline-candidate` independently regenerated the pinned
    1,624-statement candidate, reproduced SHA-256
    `fb0d371f1afe365a72bbe34e9c8fe628e318be6de0e1f43b6f73c37897c95858`,
    applied the candidate and completed `init.sql` to fresh PostgreSQL 16.14
    databases, matched normalized schema SHA-256
    `f585eb41c5d1ed62d86df2e69622c493194ef63e0706cee5602edf4149f75a44`,
    and removed its disposable containers.
  - `just policy`, `just instruction-drift`, `just docs`, and
    `just docs-link-check` passed. The link gate checked 969 links with zero
    errors, `git diff --check` reported no whitespace error, and `just ci`
    passed the complete repository gate.
  - `just ui-e2e` completed dependency audit, generated-client, browser,
    database-provisioning, application-startup, and coverage setup, then
    reproduced the inherited scheduled-profile failure recorded by ADR 542 and
    ADR 552. The unchanged test supplied raw temporary roots with
    `schedule_enabled=true`; migration 0182 returned HTTP 400 with
    `media_profile_filesystem_identity_required` and SQLSTATE `P0001` instead
    of the expected 201. The run reported 46 passed, one failed, and 61 tests
    not run; teardown consequently reported only the profile-readiness and
    job-phase read routes uncovered. No test, coverage requirement, or failure
    was weakened or skipped.
  - The final immediate-base and stack changed-line totals are recorded after
    the canonical commit-reference guards run.
- Observability updates:
  - Ignored local evidence retains the candidate digest, schema digests,
    statement boundary map, PostgreSQL identity, and successful fresh-apply
    results without credentials or database contents.
- Status-doc validation:
  - Added this task record to the ADR index and mdBook summary. Runtime and
    operator bootstrap documentation remains migration-based because cutover
    has not occurred.
- Risk & rollback plan:
  - Revert this slice to restore the inert ADR 552 prefix. No database or
    application rollback is required because no bootstrap caller uses this
    file.
- Dependency rationale:
  - No dependency was added. Assembly uses the exact PostgreSQL image, SQLx CLI,
    Ruby guard, and SHA-256 tooling recorded by ADR 528.
- Stale-policy check:
  - Reviewed `AGENTS.md`,
    `.github/instructions/revaer-data.instructions.md`,
    `.github/instructions/devops.instructions.md`, ADR 522, ADR 528, ADR 541,
    ADR 551, ADR 552, the canonical database recipes, and PR workflow.
  - The scoped devops evidence was stale at ADR 552's statement-781 prefix and
    now records the complete inert candidate. No runtime bootstrap, required
    check, Sonar criterion, or review-size limit was relaxed.
