# Media runtime cooperative shutdown

- Status: Accepted
- Date: 2026-07-29
- Context:
  - Bootstrap previously stopped media background runtimes by aborting their tasks.
  - Abort-only shutdown can strand owned media jobs until stale-worker recovery and can skip normal workspace cleanup.
- Decision:
  - Add an injected cooperative shutdown signal shared by the media discovery, media job, and media retention runtimes.
  - Request media shutdown from bootstrap before awaiting media runtime tasks, with a bounded abort fallback.
  - Route runtime shutdown into the media job worker control monitor so active inspection, transcode, and verification work use the existing durable cancellation path.
- Consequences:
  - Idle media runtimes stop without forced task abort.
  - Active media jobs receive a durable cancellation request, terminate controlled work, persist terminal cancellation state, and clean workspaces before bootstrap falls back to abort.
  - Non-media runtime shutdown remains unchanged in this slice.

## Task Record

- Motivation:
  - Close the shutdown ownership gap where a claimed media job could be aborted mid-flight without cancellation persistence or workspace cleanup.
- Design notes:
  - `runtime_shutdown` wraps a `tokio::sync::watch` channel, treats sender drop as shutdown, and provides a sleep helper for retry loops.
  - Bootstrap owns one media shutdown sender and passes cloned receivers to all media runtimes.
  - The media job runtime keeps its direct `run_tick()` test path independent from shutdown, while the spawned loop uses a shutdown-aware tick path.
  - Active job shutdown requests the existing durable job cancellation procedure before setting the in-process command/verification cancellation signal.
  - Bootstrap awaits each media runtime for `30s` before aborting and logging the fallback.
- Test coverage summary:
  - Updated discovery and retention spawn tests to request shutdown and await normal task completion instead of aborting.
  - Rust 1.96 workspace lint verifies that the shutdown-aware discovery task retains only the cancellation handle required by its drop guard.
  - Added a media job runtime test proving a pre-requested shutdown exits without claiming queued work.
  - Added media job runtime tests proving shutdown during active transcode and active verification cancels controlled work, persists `cancelled`, and cleans workspace output.
  - Bounded source, candidate, no-op, and final-state inspections share the same shutdown-aware control monitor, so an active FFprobe process cannot outlive graceful media runtime shutdown.
  - Ran `cargo test -p revaer-app watcher_only_profile_enqueues_each_durable_file_version_exactly_once --no-default-features`, `cargo test -p revaer-app spawned_runtime_runs_immediately_after_restart --no-default-features`, `cargo test -p revaer-app media_job_runtime_spawn_exits_when_shutdown_already_requested --no-default-features`, `cargo test -p revaer-app media_job_runtime_shutdown_cancels_active_transcode_and_removes_candidate --no-default-features`, `cargo test -p revaer-app media_job_runtime_shutdown_cancels_active_verification_before_replacement --no-default-features`, `cargo test -p revaer-app media_job_runtime_cancels_active --no-default-features`, and `cargo clippy -p revaer-app --no-default-features --all-targets -- -D warnings`.
  - Ran `cargo fmt --all --check`, `git diff --check`, `just policy`, and `just instruction-drift`.
  - Ran `sonar analyze secrets` on changed source and documentation files, and `sonar list issues --project VannaDii_Revaer --statuses OPEN,CONFIRMED --format table`; no open issues were reported.
  - `sonar verify --project VannaDii_Revaer --file crates/revaer-app/src/runtime_shutdown.rs` and `sonar verify --project VannaDii_Revaer --file crates/revaer-app/src/media_job_runtime.rs` were unavailable because Sonar Agentic Analysis returned HTTP 403 with Agentic Analysis disabled for the project.
  - `just ci` and `just ui-e2e` were attempted locally and blocked before project validation because the Docker daemon was unavailable and the managed local Postgres endpoint at `localhost:5432` did not become reachable.
- Observability updates:
  - Bootstrap logs when media shutdown is requested, when receivers are already gone, and when the graceful timeout falls back to abort.
- Status-doc validation:
  - Reviewed and updated `docs/adr/index.md` and `docs/SUMMARY.md` for this task record.
- Stale-policy check:
  - Reviewed instruction files: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, `.github/instructions/devops.instructions.md`, `.github/instructions/sonarqube_mcp.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
- Risk & rollback plan:
  - Risk: shutdown now requests durable cancellation for an active media job rather than leaving it for stale recovery.
  - Rollback: revert the shutdown helper, bootstrap wiring, runtime loop changes, and tests; rerun the media runtime cancellation and spawn tests.
- Dependency rationale:
  - No dependencies were added.
