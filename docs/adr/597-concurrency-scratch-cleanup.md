# Media concurrency, scratch admission and cleanup

- Status: Recorded
- Date: 2026-10-05
- Operator approval: Not applicable: implementation follows accepted ADR 594
- Supersedes: None
- Implementation status: Implemented; final CI and Linux UI E2E evidence recorded below

## Task Record

- Motivation:
  - Complete step 6 of the media recovery sequence: enforce configured worker concurrency and one active atomic replacement per source, admit jobs against aggregate scratch capacity, reclaim only Revaer-owned workspaces, and discard intermediates after their final consumer.
- Design notes:
  - Kept the existing per-root exclusive lock and local service model. The worker loop now runs claimed jobs concurrently up to the persisted policy limit and joins active workers during shutdown.
  - Added same-profile concurrency and same-source atomic-replacement admission to `media_job_worker_claim_next_v4()` in the single initializer. Scratch reservations are process-local and released after cleanup or resumable interruption; execution also monitors aggregate workspace and free-space budgets.
  - Removed consumed workspace intermediates as soon as no later writer needs them. Cleanup accepts only canonical job-attempt-claim directory keys and treats already absent paths as successful removal.
  - No dependency or new distributed coordination mechanism was added.
- Test coverage summary:
  - `just test-media-recovery` passed: 10 fingerprint, 80 media app, 363 media runtime, 5 conversion, 73 media data, and 46 runtime recovery tests. The data tests include policy-limited claims and same-source atomic replacement admission with profile concurrency configured above one.
  - Final `just ci` passed through baseline, lint, workspace tests, all-features coverage, Python coverage and release build.
  - Final `just ui-e2e` passed on the existing Debian qualification container at the same step-5 base commit. All five phases passed: 3 anonymous missing-catalog, 57 anonymous API, 3 API-key missing-catalog, 57 API-key API and 22 Chromium UI tests (142 total). SHA-256 checks confirmed all 11 changed source/test files matched the worktree. The host macOS run cannot qualify the Linux-only catalog path; the Linux run used a dedicated real catalog fixture and preserved the missing-catalog checks.
- Observability updates:
  - Retained existing outcome counters and workspace cleanup metrics. Scratch deferrals are counted separately from interrupted work; worker join, cleanup and persistence failures remain logged at their origin.
- Status-doc validation:
  - Reviewed the media instructions, `justfile`, CI workflows and ADRs 593–596. The operational command surface and accepted product status did not change. This ADR and both indexes record step 6; user-facing API and operator contracts did not change.
- Risk & rollback plan:
  - Admission remains fail-closed when scratch capacity cannot be proven. Capacity deferral requeues the same attempt without consuming a failure retry. Existing root locking protects single-process ownership; workspace cleanup skips unknown directory names. Rollback is limited to reverting this task's source and test changes on the checkpoint branch.
- Dependency rationale:
  - None added. Existing standard-library synchronization, Tokio task sets, filesystem probes, stored procedures and cleanup utilities cover the approved design.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, `.github/instructions/revaer-ui.instructions.md`, `.github/instructions/devops.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md`, plus accepted ADRs 593–596. No conflicting policy was found; no quality gate, Sonar criterion, lint or dependency exception was introduced.
