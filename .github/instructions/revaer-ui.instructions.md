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

- Keep selectors and test affordances stable. Update E2E fixtures deliberately when UI structure changes.
- Treat generated API clients and synchronized assets as generated artifacts; regenerate them intentionally and keep authored wrappers separate.
- Run every Node command through `scripts/with-node.sh`. The wrapper must select and verify the exact `.nvmrc` version so an operator's NVM-managed Node remains active rather than being replaced by a login shell.
- Every PR UI E2E shard must upload nonempty API and UI coverage with `if-no-files-found: error`. The aggregate must download all three exact named shard artifacts and run `just ui-e2e-shard-coverage` before evaluating combined route coverage; a missing shard or record is a failed gate.
- `asset_sync` must fail closed unless every required runtime SVG exists, runtime text is UTF-8, SVG files have a complete namespaced root envelope, and `crates/revaer-ui/static` contains no raster-extension assets.
- `static/revaer-logo.svg` and `static/icons/app-icon.svg` must preserve the approved purple stylized-R composition and the `revaer-purple-gradient` and `revaer-r-silhouette` identifiers; do not substitute a wordmark, palette, or symbol during asset synchronization.
- Keep emitted icon, logo, dashboard, and DataTables references rooted under `/static`; verify their targets in a Trunk release build rather than inferring paths from source layout.
- `just check-assets` must compare `crates/revaer-ui/static/nexus` from the repository root after regeneration.
- Keep legacy vendor-reference canonicalization in `asset_sync`, validate the UTF-8 served image set there, and make `just check-assets` compare the complete repository-root `crates/revaer-ui/static/nexus/**` output.
- CI E2E should use an explicit browser channel such as `E2E_BROWSER_CHANNEL=chrome` when the runner already provides that browser, so shards install Playwright dependencies without downloading redundant browser bundles. Keep CI video capture off for that path unless the Playwright ffmpeg bundle is installed.
- When UI structure, selectors, or synced assets change, update the relevant docs, tests, and instructions in the same change.
