# Pre-v1 init assembly slice 01

- Status: Recorded
- Date: 2026-08-16
- Operator approval: Not applicable: bounded implementation of accepted ADR 522
- Context:
  - ADR 522 requires the complete final-state init candidate to be assembled in
    statement-aligned pull requests that remain below the 8,500 changed-line
    assembly limit.
  - ADR 528 froze the 167-file migration corpus and proved a complete candidate
    with SHA-256
    `fb0d371f1afe365a72bbe34e9c8fe628e318be6de0e1f43b6f73c37897c95858`
    and 1,624 SQL statements.
  - Existing migrations remain the sole bootstrap authority until the later
    parity and cutover step accepted by ADR 522.
- Decision:
  - Begin tracked assembly of `crates/revaer-data/init.sql` with the exact first
    237,760 bytes of the pinned candidate.
  - End this slice at candidate statement 185 and line 7,339. The tracked prefix
    SHA-256 is
    `282015d2fd578e11b608033b468f28d0e55af8e957594fc5a199bc5b9bce0763`.
  - Change `TRANSITION_PHASE` from `freeze` to `assembly`. This requires the
    tracked init file to be a regular file, an exact candidate prefix, and a
    complete statement-boundary prefix.
  - Keep the migration directory, SQLx migration runner, application bootstrap,
    local recipes, and test setup unchanged. The partial init file is review
    evidence only and is not executable by ordinary runtime or test paths.
  - Generate the candidate independently in this worktree before materializing
    the prefix. No candidate or database dump is committed.
- Consequences:
  - Reviewers can inspect the first bounded final-state SQL section while every
    existing bootstrap path continues to use the proven migration corpus.
  - PR CI now regenerates the full candidate, verifies this exact prefix, and
    applies the prefix to an empty pinned PostgreSQL database.
  - Subsequent assembly slices may only append exact candidate bytes ending at
    another recorded statement boundary.
- Follow-up:
  - Append candidate bytes 237,760 through the next selected statement boundary
    in the immediately following stack layer.
  - Do not select `init.sql`, delete migrations, or remove transition tooling
    until the complete candidate and two-way parity proof are present.

## Task Record

- Motivation:
  - Start the accepted single-init transition without exceeding review limits
    or exposing an incomplete bootstrap authority.
- Design notes:
  - The SQL bytes are a mechanical prefix of the locally regenerated candidate;
    no SQL was hand-edited, reordered, normalized, or reinterpreted.
  - Statement boundary 185 was selected from the generated boundary map because
    its 7,339 lines leave review budget for this record and generated indexes.
- Test coverage summary:
  - `just db-init-prefix-check` accepted the 237,760-byte file as an exact,
    complete candidate prefix.
  - `just db-rebaseline-candidate` regenerated the pinned 1,624-statement
    candidate, reproduced its digest, applied the complete candidate and this
    prefix to separate empty databases, and removed its disposable container.
  - `just policy`, `just check`, `just test`, `just fmt`,
    `just instruction-drift`, and `just docs` passed. The documentation link
    gate validated all 947 links without an error.
- Observability updates:
  - Ignored local evidence retains the candidate digest, schema digests, prefix
    boundary map, and successful fresh-apply results without credentials or
    database contents.
- Status-doc validation:
  - Added this task record to the ADR index and mdBook summary. Runtime and
    operator bootstrap documentation remains migration-based because cutover
    has not occurred.
- Risk & rollback plan:
  - Revert this slice to remove the inert prefix and restore the `freeze` phase.
    No database or application rollback is required because no bootstrap caller
    uses the partial file.
- Dependency rationale:
  - No dependency was added. Assembly uses the exact PostgreSQL image, SQLx CLI,
    Ruby guard, and SHA-256 tooling recorded by ADR 528.
- Stale-policy check:
  - Reviewed `AGENTS.md`,
    `.github/instructions/revaer-data.instructions.md`,
    `.github/instructions/devops.instructions.md`, ADR 522, ADR 528, the
    canonical database recipes, and PR workflow.
  - No stale reference or contradiction was found. No runtime bootstrap,
    required check, Sonar criterion, or review-size limit was relaxed.
