# API E2E Postgres credential coherence

- Status: Accepted
- Date: 2026-08-11
- Context:
  - The API E2E job provisions Postgres with a unique per-job password but connected through a repository-level database URL.
  - The strict workflow guardrail rejects any database URL that cannot be proven to match the job's service credentials.
- Decision:
  - Compose both API E2E runtime database URLs from the same run, attempt, and job identity used by `POSTGRES_PASSWORD`.
  - Keep the admin URL construction on the same credentials and the `postgres` administration database.
- Consequences:
  - API E2E can authenticate deterministically and the job no longer depends on mutable repository variables.
  - Credentials remain isolated by workflow job.
- Follow-up:
  - Require the API E2E job and its coverage artifact to pass before handoff.

## Task Record

- Motivation:
  - Restore the API E2E job after strict credential-coherence validation exposed the mismatch.
- Design notes:
  - The change reuses the established coverage, feature-matrix, and UI-E2E credential pattern.
- Test coverage summary:
  - `just policy`, `just instruction-drift`, and `actionlint` validate structure and syntax.
  - GitHub-hosted API E2E validates the live Postgres service path.
- Observability updates:
  - The existing API E2E check and coverage artifact remain the operational signals.
- Status-doc validation:
  - Operator and API documentation were reviewed; no user-facing behavior changes.
- Risk & rollback plan:
  - Risk is limited to CI authentication. Revert this commit only with a replacement URL proven to match service credentials.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Reviewed `AGENTS.md` and `.github/instructions/devops.instructions.md`.
  - Drift was found in the API E2E job and corrected; no criteria were relaxed.
