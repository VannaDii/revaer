---
applyTo:
  - ".github/workflows/sonar.yml"
  - "sonar-project.properties"
---

# Sonar Integration

Root `AGENTS.md` defines coverage scope, local review, hosted gates, vendor ownership and consent boundaries. This file supplies integration details without adding gates or coverage targets.

## Local review

- Use project key `VannaDii_Revaer` from `sonar-project.properties`. Resolve a different requested project with `search_my_sonarqube_projects` before acting.
- Before pushing, analyze changed authored files with the project's rules. Prefer the available project/file analysis (`analyze_file_list` through SonarQube for IDE or supported project analysis); otherwise use `analyze_code_snippet` with complete changed file content, project key, explicit supported language and `MAIN` scope. It requires no PR and does not publish a scan of main.
- Compare local findings with the reviewed base; fix new findings and applicable findings in changed production code before pushing. Unchanged baseline findings in internal utilities do not create a new cleanup task. State unsupported languages, unavailable analyzers and the limits of file-only feedback; raise material gaps affecting changed production code. Do not claim local results are a full hosted analysis. MCP snippet analysis is deprecated; prefer its supported replacement when exposed, without building an analyzer or starting an integration migration as task overhead.

## Hosted analysis

- The workflows, `sonar-project.properties` and `tools/src/revaer_tooling/policy/sonar.py` define scanner configuration, inputs and exact allowlist. Use the canonical `rv sonar-*` tasks; do not copy criteria into commands or alter them remotely. Parse Java properties before validation so escaped/duplicate logical keys cannot bypass policy.
- Keep YAML inventory current for authored workflows/actions. PostgreSQL sources remain visible to text/secrets analysis; `.sql`, `.pgsql` and `.plpgsql` must not be routed to PL/SQL. The PL/SQL suffix is `.plsql` only.
- Use complete history and the exact event head/base. Reject a stale local target ref that shadows the fetched base; preserve shared worktrees. Keep configured SCM reload behavior and missing-blame checks. Debug logging changes diagnostics, not criteria.
- Set `PYTHONDONTWRITEBYTECODE=1` for scanner-job CLI processes. Source preparation removes only untracked caches, generated clients, UI build output and result mirrors; refuse tracked content or links. Measure browser execution before removing UI output.
- Preserve compiler commands and CXX headers at paths valid for the scanner checkout/SDK. Retain complete native measurements and original LLVM reports alongside the tracked Rust importer input. Use the existing adapters for same-source compilation merging and unexecuted branch normalization; do not implement alternate measurement analysis.
- The JavaScript analyzer remains active for first-party files, but no JavaScript LCOV is required when there are no production JavaScript targets. The production-source inventory guard prevents adding such code without supported coverage tooling.
- Bootstrap generic coverage comes from `rv script-coverage` and pinned, archive-verified `kcov`. Root policy defines its nonproduction status; collected records do not impose hit targets.
- Scanner-side waiting and `rv sonar-verify-result` must verify the same completed analysis. Retain the complete nonempty scanner report through the existing functional gate. Fetch paginated results; PR queries use the Sonar PR key, while main new-issue queries use `sinceLeakPeriod=true`. Do not confuse a branch name with a PR key.

## Inspection and troubleshooting

- Use `get_project_quality_gate_status` for gate status, `get_component_measures` for metrics, `search_sonar_issues_in_projects` for issues, and hotspot search/detail tools for review. Use explicit PR scope when reviewing a PR; `list_pull_requests` resolves its Sonar key.
- Server ingestion is asynchronous; issue search does not immediately reflect local fixes. Check the exact analysis task/result rather than changing dispositions.
- For authorization errors, check token type and permissions without exposing credentials. Use project discovery for lookup failures. Follow root stopping guidance when tools or analysis are unavailable.
- Runtime UI media and canonical `/static/...` URLs follow the UI instructions. Scanner success does not prove release asset resolution, and malformed first-party inputs must be fixed at their owner.
