# UI runtime image canonicalization

- Status: Accepted
- Date: 2026-08-16
- Operator approval: Explicitly approved by the operator on 2026-08-16:
  "I approve the ADRs as they are now and I'm resuming your goal."
- Context:
  - Strict all-authored-source Sonar analysis reads committed raster runtime
    assets as invalid UTF-8, and repository policy requires every scanner warning
    to be fixed without source, suffix, analyzer, coverage, or test-scope
    exclusions.
  - Revaer still needs deterministic product branding, icons, dashboard images,
    queue avatars, and manifest assets at runtime.
  - A prior SVG canonicalization implementation was prepared and pushed in the
    existing stack without decision-specific operator approval. Its presence and
    passing checks are not approval; the replacement stack must hold that
    implementation until an explicit decision is recorded. This accepted ADR is
    that decision; replay still requires implementation review against the exact
    boundary below.
- Constraints:
  - Sonar criteria may not be relaxed.
  - Runtime assets must remain license-compliant, deterministic, reviewable,
    visually validated, and available without a network dependency.
  - The asset-sync boundary must fail closed on unknown binary references and
    generated-output drift.
- Options:
  - Recommended: delete unused raster inputs and replace every required runtime
    raster with a deterministic UTF-8 SVG source, update all runtime and package
    references, and make `asset_sync` the single validated canonicalization
    boundary.
  - Keep raster assets and request an explicit, scoped Sonar binary-analysis
    exception. This preserves current imagery but weakens the operator's stated
    maximal-analysis posture and leaves binary inputs outside text analysis.
  - Generate raster assets during the build from committed text recipes. This
    keeps authored inputs scanner-readable but adds a generator/toolchain and
    generated-binary release verification surface without improving the current
    UI need over direct SVGs.
  - Fetch runtime assets from an external service. This removes repository
    binaries but adds availability, integrity, privacy, and supply-chain
    dependencies.
- Recommendation:
  - Adopt deterministic SVG canonicalization. It satisfies strict Sonar without
    an exception or new runtime/build dependency and keeps every required asset
    reviewable in the repository.
- Consequences:
  - Required UI imagery remains available as text while inherited demonstration
    photography and obsolete rasters are removed.
  - Visual appearance changes where photography is replaced by deterministic
    product illustrations; desktop and mobile visual regression review becomes
    a required acceptance gate.
  - Unknown raster references, missing generated assets, or asset-lock drift fail
    CI instead of being hidden from Sonar.
- Follow-up:
  - Replay the held implementation only after reviewing its exact
    image set, licensing, runtime references, asset-lock evidence, and local UI
    screenshots against this boundary.

## Implementation Boundary

- Approval authorizes deterministic SVG replacement of required raster runtime
  assets, deletion of unused raster inputs, reference updates, and strict
  asset-sync validation.
- Approval does not authorize Sonar exclusions, remote asset hosting, a new
  generator dependency, unrelated UI redesign, or removal of a user-visible
  image without an equivalent validated replacement.

## Task Record

- Motivation:
  - Resolve binary scanner inputs without weakening maximal Sonar analysis or
    leaving the UI visually incomplete.
- Design notes:
  - The held implementation is feasibility evidence only. This ADR accepts the
    architecture, but the implementation still requires review against the
    accepted boundary.
- Test coverage summary:
  - Require asset synchronization and unit tests, UI build, policy,
    instruction drift, full local gates, and desktop/mobile visual verification.
- Observability updates:
  - None; this changes static asset inputs and validation.
- Status-doc validation:
  - Product claims do not change. The ADR index and documentation summary already
    reference this accepted but unimplemented decision.
- Risk & rollback plan:
  - Revert references and replacements together if visual or package validation
    fails. Do not restore binary inputs through a Sonar exception without a
    separate explicit operator decision.
- Dependency rationale:
  - The accepted decision adds no dependency.
- Stale-policy check:
  - Reviewed root, UI, Sonar, and DevOps instructions. This ADR preserves
    strict analysis and the operator-approval boundary.
