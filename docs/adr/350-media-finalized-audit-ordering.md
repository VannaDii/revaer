# Media finalized audit ordering

- Status: Accepted
- Date: 2026-07-29
- Context:
  - Finalized media replacements are irreversible at the source filesystem boundary.
  - Runtime audit evidence is valuable, but audit rows are not allowed to decide terminal state after that boundary.
- Decision:
  - Complete finalized destructive replacements before writing post-finalization audit evidence.
  - Treat post-finalization phase/check rows and late-cancel evidence as explicit best-effort observability records.
  - Log any post-finalization audit write failure without demoting the already-completed job.
- Consequences:
  - Durable job status cannot say `failed` or `cancelled` because a non-terminal audit write failed after the source file was replaced.
  - Operators may see a completed finalized job with missing post-finalization evidence if the database rejects late audit rows.

## Task Record

- Motivation:
  - Close the remaining finalized-replacement gap where a transient audit write failure after filesystem finalization could send the generic runtime failure path through a completed destructive mutation.
- Design notes:
  - The runtime now calls the finalized-completion stored procedure immediately after successful replacement finalization.
  - Completion events are published after the terminal database state is recorded.
  - Post-finalization `execute`, `verify_replace`, `output_replacement`, and late-cancel evidence writes log failures at the write site and never return an error that can mark the job failed.
  - The regression test injects database audit failures through the shared disposable Postgres test-support boundary, preserving the app crate rule that SQL execution stays outside runtime wiring.
- Test coverage summary:
  - Added a media runtime regression that pauses after replacement finalization, requests cancellation, injects phase/check audit procedure failures, and verifies the job remains `completed` with no `last_error` while the source file contains the replacement bytes.
  - Ran the focused regression locally with `cargo test -p revaer-app media_job_runtime_keeps_finalized_replacement_completed_when_audit_fails --no-default-features`.
  - Ran `cargo test -p revaer-app finalized_replacement --no-default-features`, `cargo clippy -p revaer-app --no-default-features --all-targets -- -D warnings`, `cargo clippy -p revaer-test-support --all-targets -- -D warnings`, `cargo fmt --all --check`, `just policy`, `just instruction-drift`, and `git diff --check`.
  - Attempted `just ci` and `just ui-e2e`; both were locally blocked before project validation by an unavailable Docker/Postgres endpoint at `localhost:5432`.
  - Ran `sonar analyze secrets` for the changed files and `sonar list issues --project VannaDii_Revaer --statuses OPEN,CONFIRMED --format table`; Agentic Analysis `sonar verify --project VannaDii_Revaer --file ...` returned `403 Forbidden` because it is not enabled for this organization.
- Observability updates:
  - Added explicit warning logs for failed post-finalization execute phase, replacement phase, replacement check, and late-cancel evidence writes.
- Status-doc validation:
  - Reviewed and updated `docs/adr/index.md` and `docs/SUMMARY.md` for this task record.
- Stale-policy check:
  - Reviewed instruction files: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, `.github/instructions/devops.instructions.md`, `.github/instructions/sonarqube_mcp.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
- Risk & rollback plan:
  - Risk: post-finalization observability may be incomplete during a database audit-write outage.
  - Rollback: revert the runtime ordering change and this task record, then rerun the media runtime finalized-replacement tests.
- Dependency rationale:
  - No dependencies were added.
