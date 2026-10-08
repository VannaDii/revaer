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

- For the migrated root `setup.sh` only, ADR 592 permits at most one uncovered
  executable line and accepts full coverage. Preserve complete native records
  and require positive execution. This supersedes the covered-and-uncovered
  requirement below only for that bootstrap; other inputs retain their gates.

- Revaer versions its complete strict Sonar scope in `sonar-project.properties`. Keep every authored tracked top-level entry in main-code scope, use only the committed empty `.sonar-test-scope` sentinel for `sonar.tests`, and keep source, test, duplication, JavaScript, SCA, and issue-scope filters explicitly empty. Coverage concerns apply only to production code: keep the exact operator-approved coverage-only exclusions enforced by the Python Sonar policy. Do not narrow analysis or security scope.
- PostgreSQL migrations must remain visible to generic text, secrets, and main-code analysis. Do not assign `.sql`, `.pgsql`, or `.plpgsql` to the PL/SQL analyzer; reserve `sonar.plsql.file.suffixes=.plsql` for actual PL/SQL.
- Sonar property policy is enforced after Java-properties parsing. Escaped keys, leading-whitespace forms, continuations, duplicate logical keys, workflow overrides, and properties outside the exact reviewed allowlist are forbidden.
- `sonar.coverageReportPaths` must consume the generic report emitted by `rv script-coverage`. That report must come from the exact archive-hash-verified `kcov` revision installed by the canonical setup action for the retained root `setup.sh`, retain uncovered executable lines, and enforce the approved bootstrap allowance above.
- Follow the repo-wide external action versioning rule in `.github/instructions/devops.instructions.md` when editing `.github/workflows/sonar.yml`. Do not restate a conflicting Sonar-only pinning rule here.
- Revaer uses Sonar as a strict merge-control signal on pull requests. Require scanner-side waiting, retained task/report evidence, and API verification of that same analysis.
- Treat zero published coverage, a missing Rust LCOV report, a missing native LLVM coverage report, or unavailable SCM baseline data as a failed analysis even when Sonar reports a green quality gate.
- Pull-request and main-branch scans must receive the same complete measured Rust, native, browser JavaScript, Python, and root Bash bootstrap inputs. Nonproduction inputs remain available without affecting production coverage. A browser run with no authored production JavaScript may retain honest zero-hit records and must not manufacture execution. Retain the scanner report after post-scan verification.
- Use pull-request-specific quality-gate checks when the user asks whether a PR is blocked.
- New Security Hotspots on touched code must be reviewed before merge. Backlog hotspots outside touched code are tracked separately and do not automatically block unrelated work.

# Noise And Scope

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

## Python workflow execution

PR and main coverage and scanning run in the same checkout so native absolute
source paths remain valid. Use full history at the exact event SHA, the shared
coverage setup profile, and one `rv sonar-scan` invocation. Produce bootstrap
and Python coverage explicitly, merge completed E2E coverage, and retain both
coverage and complete scanner evidence even when a gate fails. Update the
complete source and YAML inventories when files change; do not narrow scope.

- Source preparation removes disposable browser HTML reports and test outputs after coverage collection. Do not copy them into a separate proof collection; retain only explicitly requested scanner outputs and required coverage inputs under the root artifact policy.
