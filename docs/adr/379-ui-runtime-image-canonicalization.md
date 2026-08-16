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
  - The deterministic SVG canonicalization implementation was independently
    reviewed after decision-specific approval and is integrated in this stack.
    The review found equivalent or stricter asset validation than the prior held
    implementation and identified one remaining release defect: repository-root
    `revaer-logo.svg` differed from the canonical runtime logo referenced by the
    chart metadata and release checklist.
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
  - Preserve the reviewed implementation's exact image set, licensing, runtime
    references, asset-lock evidence, and purple stylized-R identifiers.
  - Keep repository-root `revaer-logo.svg` byte-identical to
    `crates/revaer-ui/static/revaer-logo.svg` through the canonical asset gate.

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
  - The reviewed implementation is integrated. ADR 539 closes the isolated
    release-logo byte drift without changing any other asset or scanner rule.
- Test coverage summary:
  - ADR 539 records the validation performed for the release-logo correction.
    This record makes no independent claim for a gate not listed there.
- Observability updates:
  - None; this changes static asset inputs and validation.
- Status-doc validation:
  - Product claims do not change. The release checklist's canonical purple
    stylized-R claim now matches both committed logo locations.
- Risk & rollback plan:
  - Revert references and replacements together if visual or package validation
    fails. Do not restore binary inputs through a Sonar exception without a
    separate explicit operator decision.
- Dependency rationale:
  - The accepted decision adds no dependency.
- Stale-policy check:
  - Reviewed root, UI, Sonar, and DevOps instructions. This ADR preserves
    strict analysis and the operator-approval boundary.
