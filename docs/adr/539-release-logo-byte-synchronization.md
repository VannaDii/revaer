# Release logo byte synchronization

- Status: Recorded
- Date: 2026-08-16
- Operator approval: Not applicable: nonarchitectural task record
- Context:
  - `charts/revaer/Chart.yaml` and `docs/release-checklist.md` identify the
    repository-root `revaer-logo.svg` as the canonical purple stylized-R release
    mark.
  - Independent review of the accepted ADR 379 implementation found that the
    repository-root logo still contained an older wordmark while
    `crates/revaer-ui/static/revaer-logo.svg` contained the approved mark.
- Decision:
  - Make the repository-root logo byte-identical to the canonical runtime logo.
  - Extend `just check-assets` with a deterministic `cmp -s` check so future
    byte drift fails closed after asset synchronization.
  - Change no other UI asset, Sonar criterion, runtime reference, or visual
    design.
- Consequences:
  - Release and Artifact Hub branding now uses the same reviewed bytes as the
    runtime logo.
  - Any future logo update must change both committed locations together.
- Follow-up:
  - Keep the byte-identity assertion in the canonical asset gate.

## Task Record

- Motivation:
  - Close the single approved ADR 379 defect found by the read-only asset audit.
- Design notes:
  - A direct byte comparison is portable, deterministic, dependency-free, and
    narrower than adding another asset parser or generator.
- Test coverage summary:
  - `just check-assets`, `just check`, `just lint`, `just policy`,
    `just instruction-drift`, `just docs`, `just docs-link-check`, `just test`,
    and `just ci` passed. `git diff --check` and the direct byte comparison also
    passed.
  - The repository-owned test gates ran all 33 `asset_sync` library tests and
    its CLI test successfully.
  - `just ui-e2e` was run but did not pass on the unchanged base implementation.
    The first run stopped before browser tests because `tests/playwright.config.ts`
    references an undefined `envDir`. With that binding supplied temporarily
    outside the worktree and an isolated test database, 44 tests passed, two
    existing media API tests failed, and 60 dependent tests did not run. The
    failures include the committed test contract sending
    `POST /v1/media/jobs` while the router exposes only `GET`, plus media profile
    creation returning `400` for the test's `/tmp` paths. Those unrelated defects
    are not changed or suppressed by this logo-only task.
- Observability updates:
  - None; this changes a static release asset and its local validation gate.
- Status-doc validation:
  - Reviewed `charts/revaer/Chart.yaml` and `docs/release-checklist.md`; their
    canonical purple stylized-R claim requires no wording change after the bytes
    are synchronized.
- Risk & rollback plan:
  - Risk is limited to release-logo presentation and validation. Reverting this
    task restores the mismatch and is therefore not a safe operational fallback;
    a replacement must keep both SVG locations byte-identical.
- Dependency rationale:
  - No dependency is added or changed; `cmp` is already available in the
    repository's supported shell environments.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/revaer-ui.instructions.md`, and
    `.github/instructions/devops.instructions.md`.
  - Drift found: the scoped asset instructions did not require release-logo byte
    identity. Both scoped instruction files now state the invariant.
  - Contradictions removed: ADR 379 no longer describes the reviewed
    implementation as held or unintegrated.
