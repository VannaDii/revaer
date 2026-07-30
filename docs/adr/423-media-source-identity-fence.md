# Media source identity fence

- Status: Accepted
- Date: 2026-08-12
- Context:
  - Media discovery previously read metadata by pathname before and after opening the source. A rename, symlink swap, or same-size and same-mtime replacement could therefore make the persisted fingerprint describe different filesystem objects.
  - Claimed jobs did not carry the discovery fingerprint, so the worker could not prove that the source selected for inspection, transcoding, and replacement was the source approved at enqueue time.
- Decision:
  - Open the canonical configured source root, traverse every source-path component relative to that directory descriptor with no-follow semantics, and hash only the resulting opened regular-file handle.
  - Derive device/inode identity, size, modification time, and change time from the same handle before and after hashing. Reopen beneath the same root and require the pathname to resolve to the same identity before accepting the fingerprint.
  - Persist the complete identity tuple and SHA-256 in the discovery record and copy it into an immutable media-job intent snapshot through a database trigger. Return the snapshot from `media_job_worker_claim_next_v3`.
  - Recompute and compare the complete fingerprint when processing a claim, immediately before source inspection and command execution, and before and after replacement preparation. Any mismatch fails the job before atomic replacement.
  - Preserve legacy discovery rows with nullable identity fields. The v2 enqueue path upgrades such rows on the next observation, while the insert trigger and v3 claim path reject incomplete snapshots.
  - Alternatives considered: pathname canonicalization and metadata-only comparison remained vulnerable to swaps; deleting legacy fingerprints discarded useful history; adding a new capability dependency was unnecessary because the workspace already reviews and pins `rustix`.
- Consequences:
  - Source hashing is repeated at security-sensitive boundaries. This adds I/O, but ensures a changed source cannot be committed over after producing a candidate from stale or substituted input.
  - Unix device/inode identity and descriptor-relative open semantics are now explicit runtime requirements for the media worker.
  - Historical jobs remain readable, but an incomplete historical snapshot is not claimable for execution.
- Follow-up:
  - Run the migration-backed claim snapshot test in CI with the configured Postgres service.
  - Monitor fingerprint mismatch failures as integrity events and investigate repeated mismatches at their source roots.

## Task Record

- Motivation:
  - Resolve PR 111 review thread `PRRT_kwDOQJiaF86WPLeQ` by binding discovery, claim, execution, and replacement to one stable source identity.
- Design notes:
  - The accepted fingerprint contains lowercase `device:inode` hexadecimal identity, byte size, nanosecond modification and change timestamps, and lowercase SHA-256.
  - Descriptor-relative traversal rejects non-normal path components and symlinks at every descendant component. Metadata used for acceptance comes from opened handles, not pathname lookups.
  - Replacement preparation is discarded when the second pre-commit revalidation fails.
- Test coverage summary:
  - Added deterministic unit tests for rename-to-symlink swaps, same-size and same-mtime inode replacement, mutation during hashing, mutation after claim, stable source matching, invalid descendants, and missing roots.
  - Added a migration-backed integration test proving `claim_next` returns the exact persisted identity and hash snapshot.
  - Ran the focused fingerprint tests and whole-workspace all-target all-feature compilation. The database integration test compiled but locally skipped because no test database URL was configured.
- Observability updates:
  - Source fingerprint I/O failures and source fingerprint mismatches have stable runtime error codes and flow through the existing job failure telemetry and persisted failure path.
- Status-doc validation:
  - Reviewed the ADR index and mdBook summary and added this accepted decision. No user-facing status or operator guide required a behavioral update.
- Risk & rollback plan:
  - The main risk is additional source read I/O at execution boundaries. Roll back the commit and migration together before deploying queued jobs created with the v3 snapshot contract; do not bypass the identity checks independently.
- Dependency rationale:
  - No new third-party package was introduced. `revaer-app` now directly uses the workspace-pinned `rustix` dependency already present for media runtime process and filesystem operations.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and `.github/instructions/revaer-data.instructions.md`.
  - No policy drift or contradiction was found. No lint, Sonar, database-access, dependency, or completion criterion was relaxed.
