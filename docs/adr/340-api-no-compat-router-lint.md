# API No-Compat Router Lint

- Status: Accepted
- Date: 2026-07-29
- Context:
  - Running strict no-default app clippy compiles `revaer-api` without the qBittorrent
    compatibility routes.
  - In that feature shape, the optional compatibility mount helper simply returns the router and
    triggers the denied `clippy::missing_const_for_fn` lint.
  - With compatibility routes enabled, the helper still delegates to the non-const compatibility
    router mount.
- Decision:
  - Split the optional compatibility helper by feature configuration.
  - Keep the compatibility-enabled helper non-const.
  - Make the no-compat helper `const fn` so strict no-default clippy remains fail-closed without
    suppressions.
- Consequences:
  - Positive: no-default clippy can validate the app/API build shape without weakening lint policy.
  - Positive: compatibility-route behavior is unchanged.
  - Risk: future edits that add work to the no-compat helper may need to remove `const` or keep that
    path const-compatible.
- Follow-up:
  - Keep no-default feature validation in the normal gate set.

## Task Record

- Motivation:
  - A strict no-default app clippy pass surfaced a denied lint while validating media runtime work.
- Design notes:
  - Used feature-split helper definitions instead of source-level lint suppression.
  - Left the default compatibility route behavior untouched.
- Test coverage summary:
  - Covered by the rerun no-default clippy pass that exposed the issue.
- Observability updates:
  - None.
- Status-doc validation:
  - No operator-facing behavior changed.
- Risk & rollback plan:
  - Roll back this helper split if it causes an unexpected feature-compilation issue, then replace it
    with an equivalent no-suppression implementation.
- Dependency rationale:
  - No dependencies added.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and
    `.github/instructions/devops.instructions.md`.
  - No stale references or contradictions were found.
