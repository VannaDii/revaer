# Retired media write route contract coverage

- Status: Recorded
- Date: 2026-08-16
- Operator approval: Not applicable: test correction for accepted ADRs 359 and 362.
- Context:
  - ADR 359 removed public phase-history writes so only the media worker can
    append lifecycle evidence.
  - ADR 362 removed public job creation so admitted discovery and worker paths
    remain the only job-creation authorities.
  - The API end-to-end route inventory still treated both retired methods as
    required writable routes and rejected their intentional `405 Method Not
    Allowed` responses before later media behavior could run.
- Decision:
  - Keep both methods in end-to-end coverage, but move them into an explicit
    retired-write inventory that requires exact `405` responses.
  - Keep every supported media operation in the routed-operation inventory,
    where any `405` or server error remains a test failure.
- Consequences:
  - The end-to-end suite now agrees with the generated public contract and the
    existing Rust router tests without restoring unsafe public writes.
  - A future accidental restoration or status drift for either method fails
    the dedicated assertion.
- Follow-up:
  - Resolve the separate managed-root profile failure through accepted ADR 523;
    do not weaken the profile, discovery, or route-coverage expectations.

## Task Record

- Motivation:
  - Let the required UI gate reach implemented media workflows while preserving
    intentional worker ownership of job creation and phase history.
- Design notes:
  - This is a test-classification correction only. It does not add, remove, or
    route an HTTP operation and does not change generated API artifacts.
- Test coverage summary:
  - The focused Playwright media specification lists both retired methods and
    requires exact `405` responses under an authenticated session.
  - Supported operations continue to require a non-`405`, non-server-error
    response through the same bounded empty-request loop.
  - NVM-pinned client generation, TypeScript coverage compilation, and
    Playwright enumeration passed. A focused `api-none` run executed 47 tests:
    the retired-write assertion and 45 other tests passed, while the inherited
    profile workflow failed at its expected `201` assertion with the accepted
    root-identity contract's current `400` response. Teardown then reported the
    consequentially uncovered job-phase and profile-readiness reads. No media
    test directory remained after the run.
  - `just policy`, `just instruction-drift`, `just fmt`, and `git diff --check`
    passed. The documentation build passed through the pinned Cargo tool path
    while the separately tracked exact-tool path repair is pending integration;
    the link gate validated all 957 links without error.
- Observability updates:
  - No production observability changes are made.
- Status-doc validation:
  - ADRs 359 and 362 already document the public contract. No user or operator
    documentation advertised these retired write methods.
- Risk & rollback plan:
  - Reverting this record restores a false-negative end-to-end failure. It does
    not affect runtime behavior.
- Dependency rationale:
  - No dependency was added or changed.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/revaer-ui.instructions.md`,
    ADRs 359 and 362, the router tests, generated OpenAPI documents, and the
    Playwright media specification.
  - Drift was limited to the Playwright operation classification and the scoped
    instruction's missing statement of that contract. Both are corrected
    without relaxing route, browser, coverage, Sonar, or media assertions.
