---
applyTo:
  - ".github/workflows/pr.yml"
  - ".github/workflows/sonar.yml"
  - "just/quality.just"
  - "scripts/install-sonar-scanner.sh"
  - "scripts/prepare-sonar-scm.sh"
  - "scripts/sonar-*.sh"
  - "scripts/verify-sonar-inputs.sh"
  - "sonar-project.properties"
---

These are the repo-specific guidelines for using the SonarQube MCP server with Revaer.

# Project Defaults

- Default Sonar project key for this repository: `VannaDii_Revaer`
- If a user does not provide a project key, use `VannaDii_Revaer`.
- If a user provides a project key or seems unsure, confirm it with `search_my_sonarqube_projects` before acting.

# Tools To Use Here

- Use `get_project_quality_gate_status` to inspect overall, branch, or pull-request quality-gate status.
- Use `get_component_measures` for high-level metrics such as coverage, duplications, code smells, or hotspots.
- Use `search_sonar_issues_in_projects` to inspect open issues.
- Use `search_security_hotspots` and `show_security_hotspot` to review hotspot backlog or touched-code hotspots.
- Use `list_pull_requests` when you need Sonar pull-request identifiers for this project.
- Use `analyze_code_snippet` for local, file-scoped guidance when you have the full file content available.

# Revaer Sonar Workflow

- `sonar-project.properties` is the only scanner-criteria source. Workflow `-Dsonar.*` overrides, duplicate keys, unreviewed properties, and server-side criteria changes are forbidden.
- `sonar.sources` must exactly enumerate every tracked authored top-level entry. The committed empty `.sonar-test-scope/.gitkeep` sentinel is the only `sonar.tests` target, so tests, documentation, scripts, generated-looking first-party files, and vendored first-party sources remain in main-code scope.
- Every source, test, coverage, duplication, issue, analyzer-default, and SCA filter remains explicitly empty. Do not add exclusions, inclusions, ignored rules, `NOSONAR`, suffix suppression, bundle detection, or generated-code hiding. PostgreSQL `.sql`, `.pgsql`, and `.plpgsql` sources must not be misrouted to the PL/SQL analyzer.
- Hidden-file, text, YAML, JSON, Rust, JavaScript/TypeScript, native C-family, and available SCA analysis stay enabled. Keep full SCM reload, SCM-ignore bypass, analyzer cache, retained scanner report, and the scanner-wide 100 MB file limit. JavaScript's KB limit must remain aligned at 100000 KB.
- SCA is enabled and fail-closed. Add `sonar.sca.sbomImportPaths=release/media-compliance/media-runtime-inventory.spdx.json` only when that exact committed matching inventory exists in the integrated tree; once present, the property and inventory are mandatory together.
- Coverage must include the complete Rust workspace with all features and `--include-ffi` using exact cargo-llvm-cov 0.8.7, positive Rust LCOV, JavaScript/TypeScript LCOV, authored shell/Ruby generic coverage, a native compilation database, and retained native llvm-cov text. Hosted Linux must fail unless the `session.cpp` section contains at least one positive covered-line record; macOS may omit it only when the real native backend was not compiled.
- Use available pinned clang-19 with Rust-bundled llvm-cov and llvm-profdata. Compatibility is behavioral, not exact LLVM-major equality.
- PR and main workflows install exact SonarScanner CLI 8.1.0.6389 through `setup-revaer` only. Installation requires exact per-platform SHA-256 validation plus detached-signature verification against the committed key and pinned fingerprint. `just sonar-scan` is the one and only scanner invocation.
- Sonar checkout uses complete Git history and the exact event head. The exact event base SHA must be an ancestor of the head; divergent or stale stacked branches fail with a restack requirement before analysis.
- Reject every scanner log containing the `WARN` token after ANSI normalization. Retain one complete scanner log, one SCM evidence file, one `report-task.txt`, one submitted-report archive, one exact task ID, and the API result JSON used for verification.
- Scanner-side quality-gate waiting and API verification are both mandatory. The quality gate must evaluate without ignored conditions, coverage and line coverage must be positive, and lines-to-cover must be positive.
- PR verification scopes issues and hotspots to the pull request's new-code semantics. Main verification omits leak-period narrowing and queries all unresolved issues and all current hotspots. Hotspot status is never a filter: REVIEWED, SAFE, ACKNOWLEDGED-like, or any other disposition remains blocking.
- Zero unresolved issues and zero hotspots are required. Do not change issue or hotspot dispositions, rule activation or severity, gate conditions, new-code definitions, project settings, organization settings, or branch protection to make a run pass.
- Any scanner or server criteria relaxation requires exact operator consent naming the property or setting, scope, reason, and expiry. Silence, a prior exception, an agent-authored ADR, a green decoration, or time pressure is not consent; stop and ask instead.
- Follow the external-action, timeout, `continue-on-error`, required-context, and canonical-`just` rules in `devops.instructions.md` when changing either workflow.
- Use pull-request-specific quality-gate checks when the user asks whether a PR is blocked; use complete-project queries when assessing main.
- Scanner-readable UI runtime media must remain UTF-8 SVG. Preserve the validated Revaer purple-gradient and stylized-R identifiers; do not restore raster assets, weaken `asset_sync` validation, or add Sonar exclusions to hide malformed, off-brand, or binary runtime media.
- Treat canonical `/static/...` references and successful Trunk release-output URL checks as runtime correctness evidence; a clean source scan alone does not prove that emitted assets resolve.
- If Sonar noise comes from committed generated, vendored, or binary files, delete, regenerate, or replace the input with reviewable UTF-8 source. Do not hide the input through scanner exclusions without explicit operator consent.
- Generated, vendored, fixture, documentation, and test paths remain in authored main-code scope when they are tracked. Fix findings or obtain explicit operator consent; do not classify or exclude files to manufacture a cleaner result.

# Expectations After Fixes

- After local fixes, do not immediately assume server-side issue search has refreshed. Sonar ingestion is asynchronous.
- Use `analyze_code_snippet` for immediate local feedback, but do not treat snippet analysis as a substitute for the repository scan.

# Troubleshooting

- SonarQube requires a user token for MCP access. If you see `Not authorized`, verify token type and server permissions.
- If project discovery fails, use `search_my_sonarqube_projects` before assuming a configuration bug.
