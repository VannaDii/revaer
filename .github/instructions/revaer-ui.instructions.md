---
applyTo:
  - "crates/revaer-ui/**"
  - "tests/**"
---

`AGENTS.md` and `rust.instructions.md` apply first. This file specializes UI and E2E work.

# First-Party Vs Vendor Paths

- First-party authored UI quality targets are:
  - `crates/revaer-ui/src/**`
  - `crates/revaer-ui/i18n/**`
  - `crates/revaer-ui/tools/asset_sync/src/**`
  - `tests/**` excluding generated or installed dependencies
- Generated or vendored paths are not first-party authored UI code:
  - `crates/revaer-ui/ui_vendor/**`
  - `crates/revaer-ui/static/nexus/**`
  - `crates/revaer-ui/dist/**`
  - `crates/revaer-ui/dist-serve/**`
  - `crates/revaer-ui/target/**`
  - `tests/node_modules/**`
  - `tests/logs/**`
  - `tests/test-results/**`
- Do not hand-edit vendored or generated assets unless the task is explicitly about vendor ingestion, asset synchronization, or generated output shape.

# UI Architecture

- `app/*` is the only layer that touches browser globals, storage, router providers, or `EventSource`.
- `core/*` stays DOM-free and host-testable.
- `services/*` is transport-only. Convert DTOs into feature state before they reach UI views.
- `features/*` owns vertical slices. Features do not reach into each other directly.
- `components/*` hosts shared UI building blocks only. No persistence, API calls, or SSE side effects inside shared components.
- `models.rs` contains transport DTOs only. UI-only fields live in feature state.

# UI And E2E Maintenance

- Keep the media operator route connected to the normalized root, profile,
  association and job APIs. Conditional writes require their server ETag;
  private configuration reads must disable browser caching. Retain the media
  authoring, readiness, discovery and job-action regression tests.

- Keep the SSE store exhaustive over media lifecycle events and request refreshed
  state for those events, preserving the existing refresh behavior.

- Keep selectors and test affordances stable. Update E2E fixtures deliberately when UI structure changes.
- Media catalog save scenarios verify the successful response, completed refresh,
  and visible saved record without requiring observation of a transient disabled button.
- Treat generated API clients and synchronized assets as generated artifacts; regenerate them intentionally and keep authored wrappers separate.
- `asset_sync` must fail closed unless every required runtime SVG exists, runtime text is UTF-8, SVG files have a complete namespaced root envelope, and `crates/revaer-ui/static` contains no raster-extension assets.
- `static/revaer-logo.svg` and `static/icons/app-icon.svg` must preserve the approved purple stylized-R composition and the `revaer-purple-gradient` and `revaer-r-silhouette` identifiers; do not substitute a wordmark, palette, or symbol during asset synchronization.
- Keep emitted icon, logo, dashboard, and DataTables references rooted under `/static`; verify their targets in a Trunk release build rather than inferring paths from source layout.
- Keep the served theme CSS and asset lock synchronized with the selected vendor
  source. Preserve UTF-8 and missing-reference asset-sync regression coverage.
- `just check-assets` must compare `crates/revaer-ui/static/nexus` from the repository root after regeneration.
- Keep legacy vendor-reference canonicalization in `asset_sync`, validate the UTF-8 served image set there, and make `just check-assets` compare the complete repository-root `crates/revaer-ui/static/nexus/**` output.
- CI E2E should use an explicit browser channel such as `E2E_BROWSER_CHANNEL=chrome` when the runner already provides that browser, so shards install Playwright dependencies without downloading redundant browser bundles. Keep CI video capture off for that path unless the Playwright ffmpeg bundle is installed.
- When UI structure, selectors, or synced assets change, update the relevant docs, tests, and instructions in the same change.
- The accepted Python migration uses the selected checkout's OpenAPI document
  directly for response validation. Keep typed request inputs separate from raw
  transport data and preserve every existing API/UI assertion when porting tests.
- Keep anonymous API, authenticated API, and browser phases ordered. The default
  API executable is configured in `tests/e2e.toml`; media's library-test serving
  entry must be selected explicitly during integration and resolved from Cargo's
  actual artifact output. Never guess an executable in another target directory.
- Use native Playwright storage state and locator operations where available.
  Record failures and retries without replacing browser storage methods with
  authored JavaScript. Do not claim browser parity from runner fixture tests.
- Keep component hooks unconditional. Auth refresh, dashboard polling and SSE
  effects must retain their cleanup and authentication dependencies when views
  change; configured API origins belong in the app bootstrap layer.
- OpenAPI response validation must fail with an operation-specific diagnostic
  for missing paths, methods or statuses. Correct the contract against existing
  handlers and Serde models when drift is found; do not bypass validation to
  reproduce the old generated client's unchecked runtime behavior.
- Browser scenarios must report local HTTP 500 responses as failures. Keep
  credentials out of diagnostics. Shard aggregation must prove completed phases
  and exact scenario assignments; route files alone do not prove successful tests.

## Single-init E2E ownership

- Hosted dry-run E2E may explicitly select `E2E_MANAGED_MEDIA_ROOTS=1` to own
  private tmpfs roots and a disposable catalog. Keep real bootstrap attestation
  and missing-catalog scenarios; unmount only after all owned services stop.
  Caller-provided filesystem roots remain caller-owned.

- In the approved `feature-development` phase, Python E2E uses the complete
  initializer and sealed restricted runtime role. Require explicit test-service
  selection and verify its exact loopback binding before provisioning.
- Keep generated credentials private, bind init/drop to the resolved container
  ID, refuse malformed phase configuration, and stop every owned service before
  dropping the test database. Lifecycle/ordering fixtures are not full media
  application or browser acceptance.

- The media phase adds Python media API scenarios to both authentication phases;
  it must not replace or omit foundation scenarios. Keep the original library
  test-serving entry as the media default and preserve explicit runner overrides.
  Do not bypass production compliance startup checks to run a test suite.

- Media E2E retains missing-catalog checks in each authentication mode, then
  restarts the service to load and attest the caller's real Linux catalog after
  authentication setup resets configuration. Profile and association scenarios
  use catalog keys, immutable versions and server ETags.

- In the single-init feature phase, the Python E2E adapter adds `tests/specs/media/ui` alongside the foundation UI suite. Route-controlled media presentation tests must remain explicitly distinguished from filesystem or persistence evidence.

- `rv ui-e2e-app-test` preserves the media launch-guard and compliance library
  filters with one test thread, then the `bootstrap` integration binary, under
  default features. Require an explicit disposable test endpoint and positive
  passing execution in every group; do not count empty filter matches as coverage.
