# Bootstrap bind failure timeout slice ci

- Status: Accepted
- Date: 2026-06-01
- Context:
  - Full `just ci` failed in bootstrap integration tests that reserve an API port and expect startup to return an `ApiServer` bind error.
  - Under all features, startup performs libtorrent engine bootstrap before binding the API listener, so the previous 2-second guard could expire before the bind attempt on a loaded local run.
- Decision:
  - Keep the bind-conflict tests intact, but widen only their timeout guard to 30 seconds.
  - Leave fast configuration-validation tests on their existing 2-second guards.
- Consequences:
  - Positive outcomes:
    - The tests still fail if startup serves indefinitely, but they no longer fail before reaching the intended bind check.
    - The timeout change is isolated to tests that exercise listener bind conflicts.
  - Risks or trade-offs:
    - A true regression that misses the bind conflict can take longer to fail in these four tests.
- Follow-up:
  - Re-run the focused bootstrap test and full gates.

## Task Record

- Motivation:
  - Restore deterministic local CI after the bootstrap bind-conflict tests timed out before the API listener was reached.
- Design notes:
  - Added one shared timeout constant in the integration test file to make the longer guard explicit and scoped.
  - Did not alter production bootstrap ordering or listener behavior.
- Test coverage summary:
  - Reproduced the failing focused test with `REVAER_TEST_DATABASE_URL=postgres://revaer:revaer@localhost:5432/postgres DATABASE_URL=postgres://revaer:revaer@localhost:5432/postgres cargo test -p revaer-app --test bootstrap run_app_reads_env_database_url_and_surfaces_bind_failures -- --nocapture --test-threads=1`.
  - The focused bootstrap test and full gates will be rerun after the change.
- Observability updates:
  - None. This is test timing only.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: a real missed bind failure takes up to 30 seconds to report. Roll back by restoring the 2-second guard if startup ordering changes so the API listener binds before feature-specific bootstrap work.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
