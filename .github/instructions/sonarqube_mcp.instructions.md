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

- Revaer versions its complete strict Sonar scope in `sonar-project.properties`. Keep every authored tracked top-level entry in main-code scope and use only the committed empty `.sonar-test-scope` sentinel for `sonar.tests`. The exact source exclusions are the malformed YAML fixture and vendor package/asset paths defined by root `AGENTS.md` and enforced in `tools/src/revaer_tooling/policy/sonar.py`. Vendor code is not scanned or patched; all first-party code remains in scope. Test, duplication, JavaScript, SCA, and issue-scope filters remain explicitly empty. Apply the operator-approved production-only coverage scope from root `AGENTS.md`; do not otherwise narrow analysis or security scope.
- PostgreSQL migrations must remain visible to generic text, secrets, and main-code analysis. Do not assign `.sql`, `.pgsql`, or `.plpgsql` to the PL/SQL analyzer; reserve `sonar.plsql.file.suffixes=.plsql` for actual PL/SQL.
- Sonar property policy is enforced after Java-properties parsing. Escaped keys, leading-whitespace forms, continuations, duplicate logical keys, workflow overrides, and properties outside the exact reviewed allowlist are forbidden.
- The operator explicitly reaffirmed `sonar.scm.forceReloadAll=false` in this
  CI repair. Keep complete history, SCM analysis and the missing-blame gate;
  do not force a full reload or disable SCM to avoid warnings.
- Scanner debug logging (`--debug`) identifies malformed coverage records in the existing
  scanner log. It changes diagnostics only; scanner criteria remain canonical.
  Keep normal SCM analysis and complete checkout history; do not force a full
  blame reload for each scan.
- PR and main scanner jobs disable Python bytecode writes for every CLI process.
  Remove untracked prior-run bytecode beside authored Python sources before scanning;
  the environment setting does not remove caches from earlier runs. Verify the
  separate prepare/SCM/scan commands with the same environment locally.
- Remove untracked UI build output after browser measurement and before scanning;
  refuse tracked content rather than treating generated WASM as authored text.
- Reject a stale local target branch that shadows the exact event base. Use an
  independent checkout for local analysis rather than rewriting shared branch refs.
- The root operator-approved Rust input merge combines measured counters for the
  same physical source; preserve uncovered locations and the complete original
  compilation records. Do not drop a duplicate measurement to satisfy the importer.
- `sonar.coverageReportPaths` must consume the generic report emitted by `rv script-coverage`. That report must come from the exact archive-hash-verified `kcov` revision installed by the canonical setup action for the retained root `setup.sh`, retain uncovered executable lines, and enforce the approved bootstrap allowance above.
- Follow the repo-wide external action versioning rule in `.github/instructions/devops.instructions.md` when editing `.github/workflows/sonar.yml`. Do not restate a conflicting Sonar-only pinning rule here.
- Revaer uses Sonar as a strict merge-control signal on pull requests. Require scanner-side waiting, retained task/report evidence, and API verification of that same analysis.
- The operator-approved Rust scanner input is `coverage/rust-sonar-lcov.info`: retain every tracked Rust source record from the complete LLVM report and translate unexecuted `BRDA` markers to zero hits without removing branches. Third-party and untracked generated records remain in the complete original `coverage/lcov.info` and native reports. Preserve all thresholds and authored analysis scope; reject untracked checkout Rust sources outside generated `target/` output.
- ADR 592 records the operator-approved main post-scan contract: use the
  supported `sinceLeakPeriod=true` API query and require zero new unresolved
  issues and zero new hotspots. PR checks still require zero current issues and
  hotspots in that PR. Do not filter hotspot dispositions or change scanner
  scope, server criteria, coverage thresholds, or warning failures.
- Treat zero published coverage, a missing Rust LCOV report, a missing native LLVM coverage report, or unavailable SCM baseline data as a failed analysis even when Sonar reports a green quality gate.
- Pull-request and main-branch scans must receive the same complete Rust, native, browser JavaScript, Python, and root Bash bootstrap coverage inputs and must retain the scanner report as uploaded evidence after post-scan verification.
- Use pull-request-specific quality-gate checks when the user asks whether a PR is blocked.
- Check zero new unresolved issues, zero unresolved historical issues in active production code, and zero hotspots in the PR or main new-code scope. Fetch the complete unresolved issue backlog with pagination. PR queries use explicit PR scope; main new-issue queries use `sinceLeakPeriod`. Historical internal utility issues and reviewed hotspots do not block the production gate.
- Preserve real browser LCOV records even when the application executes no authored production JavaScript. Require execution when production JavaScript exists; do not manufacture hits or require an arbitrary number of lines.

# Noise And Scope

- Scanner-readable UI runtime media must remain UTF-8 SVG. Preserve the validated Revaer purple-gradient and stylized-R identifiers; do not restore raster assets, weaken `asset_sync` validation, or add Sonar exclusions to hide malformed, off-brand, or binary runtime media.
- Treat canonical `/static/...` references and successful Trunk release-output URL checks as runtime correctness evidence; a clean source scan alone does not prove that emitted assets resolve.
- If Sonar noise comes from committed generated, vendored, or binary files, delete, regenerate, or replace the input with reviewable UTF-8 source. Do not hide the input through scanner exclusions without explicit operator consent.
- First-party generated, fixture, documentation, and test paths remain in authored main-code scope when tracked. Vendor paths follow the exact root-policy exclusions. Fix findings or obtain explicit operator consent; do not classify or exclude files to manufacture a cleaner result.

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

Local PR qualification supplies only explicit pull-request identity (key, base,
head branch) to the scanner; quality criteria remain exclusively in
`sonar-project.properties`. Local analysis must never default to the main branch.

- Vendor-free Rust and JavaScript scanner LCOV inputs are derived by
  `sonar-verify-inputs`. Keep complete measured original reports and native records;
  retain every first-party counter unchanged in the scanner inputs.
