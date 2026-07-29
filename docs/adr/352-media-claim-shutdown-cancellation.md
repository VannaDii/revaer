# Media claim shutdown cancellation

- Status: Accepted
- Date: 2026-07-29
- Context:
  - Cooperative media runtime shutdown stops idle workers and active command or verification work.
  - A shutdown request can still race with a worker tick after stale recovery checks but before or immediately after claiming the next queued job.
  - Starting workspace creation and preflight for newly claimed work during shutdown makes service stop less deterministic and can leave fresh jobs dependent on later cancellation paths.
- Decision:
  - Recheck the injected runtime shutdown signal after `media_job_worker_claim_next_v2` returns a job.
  - If shutdown is already requested, request durable job cancellation and acknowledge it through the existing worker cancellation audit path before any workspace is created.
  - Keep normal non-shutdown job processing unchanged.
- Consequences:
  - Runtime shutdown no longer starts fresh media job work after a claim race.
  - Claimed jobs cancelled during shutdown receive the same terminal `cancelled` state and cancellation audit evidence as active operator-cancelled jobs.
  - If the database cancellation or audit write fails, the worker reports a tick failure and stale-worker recovery remains the fallback.

## Task Record

- Motivation:
  - Close the remaining shutdown race where a stopping media worker could claim queued work and begin normal processing after shutdown had already been requested.
- Design notes:
  - `run_tick_with_shutdown` routes claimed jobs through a shutdown-aware helper instead of directly entering `process_job`.
  - The shutdown helper avoids workspace creation and calls the existing durable cancel and worker acknowledgment procedures.
  - Existing telemetry records a `cancelled` job outcome for shutdown-cancelled claims.
- Test coverage summary:
  - Added a regression test for the claim-then-shutdown ordering, proving the job becomes `cancelled`, source bytes remain unchanged, cancellation audit evidence is present, and no job workspace is created.
  - Ran `cargo test -p revaer-app media_job_runtime_shutdown_after_claim_cancels_without_workspace --no-default-features`.
  - Ran `cargo clippy -p revaer-app --no-default-features --all-targets -- -D warnings`.
  - Ran `cargo fmt --all --check`, `git diff --check`, `just policy`, and `just instruction-drift`.
  - Ran `sonar analyze secrets` on changed source and documentation files, and `sonar list issues --project VannaDii_Revaer --statuses OPEN,CONFIRMED --format table`; no open issues were reported.
  - `sonar verify --project VannaDii_Revaer --file crates/revaer-app/src/media_job_runtime.rs` was unavailable because Sonar Agentic Analysis returned HTTP 403 with Agentic Analysis disabled for the organization.
  - `just ci` and `just ui-e2e` were attempted locally and blocked before project validation because the Docker daemon was unavailable and the managed local Postgres endpoint at `localhost:5432` did not become reachable.
- Observability updates:
  - Added an info log when a claimed job is cancelled before shutdown.
- Status-doc validation:
  - Reviewed and updated `docs/adr/index.md` and `docs/SUMMARY.md` for this task record.
- Stale-policy check:
  - Reviewed instruction files: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, `.github/instructions/devops.instructions.md`, `.github/instructions/sonarqube_mcp.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
- Risk & rollback plan:
  - Risk: shutdown-raced claimed jobs now become terminally cancelled immediately instead of remaining claimed for later runtime processing.
  - Rollback: revert the shutdown-aware claimed-job helper and regression test; rerun the focused media job runtime shutdown tests.
- Dependency rationale:
  - No dependencies were added.
