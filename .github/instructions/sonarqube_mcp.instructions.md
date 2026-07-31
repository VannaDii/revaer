---
applyTo:
  - ".github/workflows/sonar.yml"
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

- Revaer versions Sonar analysis scope in `sonar-project.properties`. Treat that file as the source of truth for complete tracked-entry scope, SCM-ignore exclusion disabling, full SCM blame reload, enabled cross-file analyzer cache support, retained complete scanner-report evidence, strict empty scope-filter overrides, binary suffix exclusion prohibition, exact HTML, YAML, and Kubernetes language ownership, Rust Clippy execution, Rust LCOV import, JavaScript/TypeScript LCOV import, native compile database import, staged native CXX bridge header imports, JavaScript committed-asset analysis, generic YAML/JSON IaC analysis, SQL dialect suffix separation, SCA dependency-resolution strictness, CFamily SCA, committed SPDX import, and quality-gate waiting. `sonar.sources` must enumerate every authored tracked top-level file or directory and must not scan the untracked `.git` object database. `sonar.tests` must point only to the committed empty `.sonar-test-scope` sentinel so Sonar cannot heuristically classify `docs`, `tests`, or test-named authored files out of main-code rules and secrets analysis. Leave `sonar.kubernetes.file.suffixes` unset, assign every tracked Helm template manifest exactly through `sonar.lang.patterns.kubernetes`, and assign every other tracked YAML file exactly through the disjoint `sonar.lang.patterns.yaml`; the workflow guardrail must fail when a tracked YAML file is unclassified or multiply classified. SonarCloud's IaC Kustomization sensor currently applies content predicates to every MAIN input before language-specific analysis and may attempt to decode binary assets as UTF-8; do not hide that upstream defect with source or suffix exclusions, SCM disablement, alternate source encoding, test misclassification, or analyzer deactivation without explicit operator consent.
- Sonar findings are work items, not noise to suppress. Do not add, broaden, or reintroduce Sonar issue ignores, rule-scope restrictions, source/test inclusions or exclusions, coverage exclusions, duplication exclusions, analyzer default exclusions, binary suffix exclusions, bundle/generated-code skips, disabled available analyzers, SCA exclusions, SCA no-resolve settings, manifest-failure tolerance, SCM disablement, SCM-ignore-based hiding, scanner skip switches, quality-gate relaxation, `NOSONAR`, or comparable suppressions without explicit operator consent recorded in the task ADR. `sonar-project.properties` is the sole criteria source; workflow overrides and duplicate keys are prohibited. Every accepted property must be present on the exact workflow-guardrail allowlist, and an agent MUST review the complete property semantics before updating that allowlist, even when the proposed value appears stricter. Do not move authored first-party files out of the source gate to make findings disappear, and do not rely on `.gitignore`, analyzer default dependency-directory exclusions, generated-code detection, or scanner defaults to hide files that should be fixed. Routing files into the wrong language analyzer, weakening analyzer activation, or accepting scanner warnings as normal is Sonar criteria relaxation. Convenience, scanner noise, time pressure, generated-code discomfort, vendored-code discomfort, binary assets, duplication discomfort, coverage discomfort, dependency-resolution failures, a skipped scan, skipped entitled dependency analysis, missing coverage inputs, missing native analyzer inputs, disabled analyzer feature flags, missing SCM blame, deprecated analysis parameters, quality-of-analysis warnings, scanner `WARN` lines, or a desire to make CI pass are never valid reasons to relax Sonar. Fix the finding, add real coverage, supply the precise analyzer input, delete or regenerate the offending artifact, or stop and obtain explicit operator consent before weakening the gate. The same prohibition governs Sonar MCP, API, and UI mutations: never weaken quality-profile rule activation or severity, quality-gate conditions or thresholds, the new-code definition, project or organization settings, issue or hotspot dispositions, or branch-protection requirements to obtain a pass. Before any relaxation, the agent MUST stop and obtain a new operator statement naming the exact property or remote setting, scope, reason, and expiry; neither an agent-authored ADR nor consent inferred from schedule pressure, prior exceptions, or a request to make checks pass is authorization. Without that statement, do not mark findings accepted or false-positive, do not mark hotspots safe or acknowledged, and do not mutate server-side analysis criteria.
- Revaer does not map PostgreSQL procedure or migration suffixes such as `.sql`, `.pgsql`, and `.plpgsql` into Sonar's Oracle PL/SQL analyzer. PostgreSQL files remain in repository-root scan scope and are validated through migration/test gates; if Sonar adds an exact PostgreSQL analyzer for this repository path, enable that analyzer directly instead of borrowing an incompatible dialect.
- PL/SQL suffix mapping must stay limited to `.plsql` for actual Oracle PL/SQL files. Keep `sonar.plsql.defaultSchema=public` as the explicit default if such files are introduced. Do not configure placeholder PL/SQL data-dictionary JDBC settings; Sonar's data-dictionary mode queries Oracle dictionary views and is only valid with an intentional Oracle dictionary.
- Rust unit and integration tests may live under `src/**/tests*` as well as crate-level `tests/`; do not hide duplication findings in those paths. Fix the duplication or obtain explicit operator consent before adding an exception.
- Rust workspace members share the repository-root `Cargo.lock`; do not hide manifest findings in member `Cargo.toml` files. Keep lockfile and manifest handling explicit enough for Sonar to pass.
- Follow the repo-wide external action versioning rule in `.github/instructions/devops.instructions.md` when editing `.github/workflows/sonar.yml`. Do not restate a conflicting Sonar-only pinning rule here.
- Revaer uses Sonar as a strict merge-control signal on pull requests. Keep both PR quality-gate status decoration and scanner-side quality-gate waiting active.
- Sonar's PostgreSQL service container must keep its explicit shared-memory setting aligned with `.github/instructions/devops.instructions.md` and `just db-start`, because the analysis workflow runs migration-backed coverage inputs.
- Use pull-request-specific quality-gate checks when the user asks whether a PR is blocked.
- New Security Hotspots on touched code must be reviewed before merge. Backlog hotspots outside touched code are tracked separately and do not automatically block unrelated work.
- Every CI scan must finish with `just sonar-verify-result`. That gate must fail when report-task metadata or the complete submitted scanner-report archive is absent or empty, when Sonar omits positive coverage, line coverage, or lines-to-cover metrics, when any quality-gate condition is ignored, or when the current new-code scope retains an unresolved issue or unreviewed hotspot. A successful scanner process or green decoration without this evidence is not a valid Sonar pass.

# Noise And Scope

- Generated, vendored, binary, or transient paths committed to the repository remain visible to Sonar unless an explicit operator-approved ADR records a task-scoped exception. Analyzer defaults that would skip committed JavaScript dependencies, bundles, generated code, or over-default-size files must be overridden in the strict direction. Binary suffix exclusions are forbidden without new operator consent. Keep hidden-file scanning, text analysis, and the repository-wide additional text scope explicitly enabled.
- CI-generated ignored outputs must stay in the coverage-producing job. The Sonar scan job must start from a clean checkout, fetch the actual pull-request base branch for PR analysis, verify that the fetched ref resolves to `github.event.pull_request.base.sha`, install `libtorrent-rasterbar-dev` for the system headers referenced by the native compile database, and download only the prepared Rust LCOV, JavaScript/TypeScript LCOV, native compile database, and staged CXX bridge-header inputs; do not use SCM ignore behavior as an analysis escape hatch.
- If Sonar noise appears to come from first-party authored code, fix the code. If it appears to come from generated, vendored, binary, or transient files, prefer deleting, regenerating, replacing the artifact with UTF-8 source assets, or using an exact matching analyzer configuration over adjusting `sonar-project.properties`; changing scan criteria still requires explicit operator consent recorded in the ADR.

# Expectations After Fixes

- After local fixes, do not immediately assume server-side issue search has refreshed. Sonar ingestion is asynchronous.
- Use `analyze_code_snippet` for immediate local feedback, but do not treat snippet analysis as a substitute for the repository scan.

# Troubleshooting

- SonarQube requires a user token for MCP access. If you see `Not authorized`, verify token type and server permissions.
- If project discovery fails, use `search_my_sonarqube_projects` before assuming a configuration bug.
