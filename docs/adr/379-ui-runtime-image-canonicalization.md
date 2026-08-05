# UI Runtime Image Canonicalization

- Status: Accepted
- Date: 2026-08-04
- Context:
  - PR 130 removes unused Nexus `public/images`, but the runtime and vendor input trees still contain raster images that strict all-authored-source Sonar analysis reads as invalid UTF-8.
  - Revaer policy forbids hiding committed binary assets through source, suffix, test-scope, or analyzer exclusions without explicit operator consent.
  - The served UI needs deterministic dashboard thumbnails, queue avatars, app identity, and Revaer branding, not the inherited Nexus demonstration photography.
- Decision:
  - Remove tracked raster inputs under the Nexus `html/images` and served `static/nexus/images` trees.
  - Replace required product, dashboard, avatar, icon, and decorative assets with deterministic UTF-8 SVG files and update every runtime, chart, manifest, and documentation reference.
  - Keep legacy vendor-reference canonicalization in `asset_sync`, validate the served UTF-8 image set, and retain deterministic asset-lock metadata.
  - Fix `just check-assets` so it compares `crates/revaer-ui/static/nexus` from the repository root after synchronization.
  - Do not change Sonar analysis scope, suffix handling, test classification, or exclusions.
- Consequences:
  - Required UI images remain available as reviewable text inputs while obsolete inherited raster assets are deleted.
  - The asset synchronizer becomes the single transformation boundary for vendor references and fails when unknown raster references or generated-output drift return.
  - Some inherited Nexus demonstration imagery is intentionally absent; restoring it requires a reviewed UTF-8 replacement rather than a scanner exception.
- Follow-up:
  - Re-run strict PR analysis after this layer is placed below the quality baseline and fix any remaining scanner warning at its source.

## Task Record

- Motivation:
  - Eliminate committed binary scanner inputs without weakening analysis and keep the runtime UI visually complete with deterministic assets.
- Design notes:
  - `asset_sync` copies the required vendor CSS and JavaScript, validates known legacy DataTables avatar literals, injects the served SVG avatar canonicalizer, validates the committed runtime image directory, and writes deterministic lock statistics.
  - Dashboard helpers resolve SVG paths. The shell, web manifest, browser configuration, Helm chart metadata, and release documentation point at SVG brand assets.
  - The generated JavaScript delta is limited to the runtime avatar override; vendor source data remains unchanged to avoid duplicated generated blocks.
- Test coverage summary:
  - `just check-assets`
  - `cargo test -p asset_sync`
  - `cargo test -p revaer-ui logic::tests::icon_sources_rotate --all-features`
  - `cargo clippy -p asset_sync -p revaer-ui --all-targets -- -D warnings`
  - `just fmt`
  - `just policy`
  - `just instruction-drift`
  - `git diff --check`
- Observability updates:
  - No runtime logging, metrics, tracing, health, or event surface changes.
- Status-doc validation:
  - Updated ADR 031, the release checklist, the ADR index, and the documentation summary for the current SVG asset contract.
- Risk & rollback plan:
  - Roll back the SVG references and assets together if visual verification finds a regression. Do not restore binary inputs on a Sonar-scanned branch without explicit operator consent.
- Dependency rationale:
  - No dependency was added or changed.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/revaer-ui.instructions.md`, `.github/instructions/sonarqube_mcp.instructions.md`, and `.github/instructions/devops.instructions.md`.
  - Drift was found in the cwd-relative `check-assets` comparison and the lack of an explicit UTF-8 canonicalization boundary; both are corrected in this layer.
