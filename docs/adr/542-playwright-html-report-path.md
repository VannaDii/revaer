# Playwright HTML report path correction

- Status: Recorded
- Date: 2026-08-16
- Operator approval: Not applicable: nonarchitectural task record.
- Context:
  - The Playwright HTML reporter joins its output directory to `envDir`, but a
    prior configuration cleanup removed the declaration while leaving the
    reference in place.
  - Loading the instrumented configuration therefore throws a reference error
    before Playwright can enumerate or execute any API or browser project.
  - The UI gate already supplies `E2E_ENV_DIR` as the repository `tests`
    directory so generated reports remain at `tests/playwright-report` even
    when TypeScript is compiled beneath `target/js-coverage-tests`.
- Decision:
  - Restore the resolved environment-directory binding from `E2E_ENV_DIR`, with
    the configuration directory as the direct-invocation fallback.
  - Keep the reporter list, HTML reporter options, project graph, browser
    matrix, retries, workers, and coverage metadata unchanged.
- Consequences:
  - Playwright configuration loading no longer fails before required checks can
    start.
  - The HTML report continues to be written to `tests/playwright-report` under
    the repository `just ui-e2e` gate.
- Follow-up:
  - Keep focused configuration listing in the UI validation loop so unresolved
    reporter-path identifiers fail before the full end-to-end run.

## Task Record

- Motivation:
  - Restore execution of the required UI end-to-end check without weakening or
    bypassing any project, browser, coverage, or reporting requirement.
- Design notes:
  - The repair restores one missing constant and reuses the existing
    `E2E_ENV_DIR` contract. It does not change test selection or runtime state.
- Test coverage summary:
  - The coverage-target TypeScript configuration compiled through the
    NVM-pinned Node wrapper.
  - Focused configuration assertions verified the exact
    `tests/playwright-report` path, both API projects, Chromium, Firefox, and
    WebKit projects, and every project coverage-file mapping. Playwright listed
    134 tests across 34 files without a configuration error.
  - `just ui-e2e` passed dependency audit, client generation, browser setup,
    configuration loading, database provisioning, application startup, report
    generation, and JavaScript coverage generation. It then failed on separate
    inherited media API behavior. The stale `POST /v1/media/jobs` operation
    returned 405 and failed the existing `expect(...).not.toBe(405)` assertion
    at `tests/specs/api/media.spec.ts:76`. Profile creation returned HTTP 400
    with problem detail `failed to upsert media profile`, error code
    `media_profile_filesystem_identity_required`, and SQLSTATE `P0001` instead
    of the expected 201. Teardown reported eight consequentially uncovered
    media routes. No test or coverage requirement was weakened or skipped.
- Observability updates:
  - The existing list reporter and HTML report remain enabled; the HTML artifact
    retains its established `tests/playwright-report` location.
- Status-doc validation:
  - Product and operator documentation are unaffected. The ADR index,
    documentation summary, generated documentation manifests, and scoped UI
    instruction are refreshed.
- Risk & rollback plan:
  - Risk is limited to resolving the reporter output root. Reverting the single
    binding restores the startup failure and is therefore appropriate only if
    the reporter no longer references that directory.
- Dependency rationale:
  - No dependency was added or changed.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/revaer-ui.instructions.md`,
    `just/ui.just`, `scripts/with-node.sh`, and the Playwright configuration.
  - Operational drift was found because the scoped UI instruction did not state
    how the coverage-compiled configuration preserves the stable HTML report
    root. The instruction now records that binding while retaining its exact
    NVM selection, strict coverage, and nonempty artifact requirements.
