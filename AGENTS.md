# AGENT.MD — Codex Operating Instructions (Revaer, Rust 2024)

> **Prime Directives**
>
> 1. **Rust 2024 only**. Never lower the edition.
> 2. **No dead code**. No unused items, no future stubs, no parking-lot code.
> 3. **Minimal dependencies**. Prefer `std`; every new dependency needs written rationale.
> 4. **The `rv` CLI is canonical**. Local and CI build/test/lint/release gates run through the [Python task registry](./tools/src/revaer_tooling/cli.py).
> 5. **Stored procedures or bust**. Runtime database access goes through stored procedures; raw SQL belongs only in migrations and tightly scoped operational bootstrap scripts.
> 6. **Deterministic, panic-free production code**. No `panic!`, `unwrap()`, `expect()`, `unreachable!()`, or silent error suppression in authored production or bootstrap code.
> 7. **No source-level lint suppressions**. `#[allow(...)]` and `#[expect(...)]` are not permitted in authored code, except for the operator-approved generated CXX documentation exception defined in the FFI instructions.
> 8. **`rv ci` and `rv ui-e2e` before every hand-off**. A task is not complete until both pass cleanly.
> 9. **Dependencies are injected**. Runtime logic receives collaborators from callers; only bootstrap/wiring code constructs concrete implementations or reads the environment.
>
> **Completion Rule:** Because Codex runs locally, a task is complete only when all requirements in this file and the scoped instruction files are satisfied, `rv ci` passes without warnings or errors, and `rv ui-e2e` passes.

---

## 0) Policy Precedence And Source Of Truth

- [`AGENTS.md`](./AGENTS.md) is the non-negotiable root contract.
- Scoped instruction files under [`.github/instructions/`](./.github/instructions/) may only tighten or specialize the root contract for their matching paths. They may not relax root policy.
- Product documentation, generated documentation indexes, dated plans, ADR implementation notes, examples, and test fixtures are reference material, not independent authorization or agent operating rules. Preserve accepted product decisions; check the source's status, date, and current implementation before treating historical completion claims or next steps as current work.
- If two instruction files appear to conflict, precedence is:
  1. [`AGENTS.md`](./AGENTS.md)
  2. the most specific scoped instruction file
  3. supporting docs and ADRs
- Operational source-of-truth files are:
  - [the `rv` task registry](./tools/src/revaer_tooling/cli.py)
  - [`.github/workflows/ci.yml`](./.github/workflows/ci.yml)
  - [`.github/workflows/pr.yml`](./.github/workflows/pr.yml)
  - [`.github/workflows/sonar.yml`](./.github/workflows/sonar.yml)
  - [`sonar-project.properties`](./sonar-project.properties)
- This file must reference those operational files instead of copying large command bodies or stale workflow inventories.
- Current scoped instruction files:
  - [`.github/instructions/python.instructions.md`](./.github/instructions/python.instructions.md)
  - [`.github/instructions/rust.instructions.md`](./.github/instructions/rust.instructions.md)
  - [`.github/instructions/revaer-data.instructions.md`](./.github/instructions/revaer-data.instructions.md)
  - [`.github/instructions/revaer-ui.instructions.md`](./.github/instructions/revaer-ui.instructions.md)
  - [`.github/instructions/ffi.instructions.md`](./.github/instructions/ffi.instructions.md)
  - [`.github/instructions/devops.instructions.md`](./.github/instructions/devops.instructions.md)
  - [`.github/instructions/sonarqube_mcp.instructions.md`](./.github/instructions/sonarqube_mcp.instructions.md)

---

## 1) Repository Invariants

- Keep the repo library-first. Binaries are thin bootstrap/wiring layers; reusable logic lives in library crates.
- Keep the public API small. Use `pub(crate)` by default and expose cross-crate items intentionally.
- Runtime database access uses stored procedures only. No runtime inline SQL outside the migration/operational exceptions called out in scoped instructions.
- `JSONB` and other conglomerate persistence formats are banned for application state. Persist normalized data.
- Runtime collaborators are injected. Do not read environment variables or construct concrete infra implementations inside domain logic.
- Zero dead code is mandatory. If code ships, it is exercised in production, tests, or an explicitly exercised feature configuration.
- Advisory ignores are forbidden by default: [`.secignore`](./.secignore), the advisory-ignore list in [`deny.toml`](./deny.toml), and Python audit exceptions must remain empty. Cargo and Python findings at every configured severity are mandatory fixes. Any exception requires explicit operator consent recorded in the task ADR, an expiry, and a guardrail change in the same commit. Exact duplicate-crate tolerances in `deny.toml` must be ADR-backed, time-bounded, and removed as soon as the graph can converge.

---

## 2) Authored Code Quality Posture

- Edition and MSRV are pinned by [`Cargo.toml`](./Cargo.toml) and [`rust-toolchain.toml`](./rust-toolchain.toml). Keep them aligned.
- Treat warnings as errors across the workspace. Do not weaken lint posture in source or ad hoc commands.
- Authored production and bootstrap code must not panic. Return errors explicitly and terminate cleanly at the top-level boundary.
- Silent error suppression is forbidden. Handle, translate, or propagate every fallible operation.
- Log errors at their origin point once. Do not re-log the same error as it travels up the call chain.
- `Option<T>` is allowed only for legitimate absence semantics or partial-function domains where `None` is the complete, expected result.
- `Result<T, E>` is required for recoverable failure. Do not hide failure in `Option`, booleans, sentinel values, or logs.
- `std::panic::catch_unwind` is forbidden everywhere except documented FFI boundary shims covered by [`.github/instructions/ffi.instructions.md`](./.github/instructions/ffi.instructions.md).
- If a rule cannot be satisfied cleanly, redesign, split, delete, or isolate the code behind the documented FFI boundary. Do not silence the rule. The sole approved generated-code exception is `clippy::missing_errors_doc` on the CXX bridge module, as scoped in [`.github/instructions/ffi.instructions.md`](./.github/instructions/ffi.instructions.md); authored Rust remains subject to the lint.

---

## 3) Quality Gates

- Operator-approved Sonar policy: coverage requirements concern production code only. Keep the exact coverage-only exclusions enforced by `tools/src/revaer_tooling/policy/sonar.py`; all authored content remains in analysis and the zero-new-issues gate. No unresolved historical issue may remain in active production code, including the runtime SQL initializer and used vendored libraries. Only `TO_REVIEW` hotspots block; reviewed hotspots do not. Real browser records may have zero hits when no authored production JavaScript is executed. This specific coverage policy supersedes the blanket empty-coverage-filter language below; all other analysis restrictions remain in force.

- All local and CI operations run through `rv` tasks. Workflows may install tools or stage artifacts, but build/test/lint/release gates must call `uv run --locked -- rv`.
- Bootstrap once with `./setup.sh`; use the uv-managed `rv` CLI thereafter. The approved migration retires Just, Node tooling and shell automation except this root bootstrap.
- Handoff requires:
  - `rv ci`
  - `rv ui-e2e`
- [the `rv` task registry](./tools/src/revaer_tooling/cli.py) is the canonical command surface for build, lint, test, coverage, release, docs, and local dev loops.
- [`ci.yml`](./.github/workflows/ci.yml), [`pr.yml`](./.github/workflows/pr.yml), and [`sonar.yml`](./.github/workflows/sonar.yml) must stay aligned with the `rv` registry and this policy.
- Sonar analysis scope and first-party signal shaping are versioned in [`sonar-project.properties`](./sonar-project.properties). `sonar.sources` must enumerate every authored tracked top-level repository entry so all authored content remains in main-code scope without treating the untracked `.git` object database as source. `sonar.tests` must point only to the committed empty `.sonar-test-scope` sentinel: this disables Sonar's path heuristic, prevents `docs` and `tests` paths from silently skipping secrets analysis, and keeps every authored test under main-code rules and coverage. Strict empty scope filters are intentional scanner-side overrides of any project-side narrowing. Rust coverage must exercise the complete workspace with all features, Python coverage must come from executed tooling and E2E code, browser JavaScript coverage must come from Playwright execution, and all coverage streams must retain real source and line records, and Sonar must publish positive coverage and lines-to-cover metrics.
- Sonar strictness is mandatory and fail-closed. Do not add, broaden, or reintroduce Sonar issue ignores, rule-scope restrictions, source/test inclusions or exclusions, coverage exclusions, duplication exclusions, analyzer default exclusions, binary suffix exclusions, bundle/generated-code skips, dependency-analysis degradation, disabled available analyzers, disabled or unavailable analyzer caches that reduce cross-file analysis, partial SCM reload, discarded scanner evidence, file-size lowering, SCM disablement, SCM-ignore-based hiding, scanner skip switches, quality-gate relaxation, `NOSONAR`, or comparable suppressions without explicit operator consent recorded in the task ADR. `sonar-project.properties` is the only permitted scanner-criteria source: workflow overrides, duplicate keys, and unreviewed properties are forbidden, and every accepted property must remain on the exact allowlist enforced by [`tools/src/revaer_tooling/policy/sonar.py`](./tools/src/revaer_tooling/policy/sonar.py). Do not classify any authored first-party file as test-only or out of the source gate to make findings disappear, alter or populate `.sonar-test-scope`, restore automatic test-path detection, route files into the wrong language analyzer, or remove coverage instrumentation to manufacture a cleaner log. The post-scan gate must prove that positive coverage and lines-to-cover metrics were published, the quality gate evaluated without ignored conditions, no new unresolved issue or unreviewed hotspot remains, and the complete submitted scanner report was retained as nonempty evidence. Do not use vendored/generated discomfort, binary assets, scanner noise, time pressure, coverage discomfort, dependency-resolution failures, or a desire to make CI pass as an excuse to relax Sonar. A failing Sonar gate, skipped scan, skipped entitled dependency analysis, missing coverage input or metric, missing native analyzer input, disabled analyzer feature flag, unavailable analyzer cache, missing SCM blame, quality-of-analysis warning, deprecated analysis parameter, absent scanner evidence, or scanner `WARN` line is a required fix, not permission to hide files or soften criteria. Fix the finding, add real coverage, supply the precise analyzer input, delete or regenerate the offending artifact, or stop and obtain explicit operator consent before weakening the gate. This prohibition applies equally to repository files and Sonar server state: agents MUST NOT weaken quality-profile rule activation or severity, quality-gate conditions or thresholds, the new-code definition, project or organization analysis settings, issue or hotspot dispositions, or branch-protection requirements as a substitute for fixing code. Agents MUST stop before making any criteria-relaxing edit or remote mutation and ask the operator for consent that names the exact property or server setting, scope, reason, and expiry. Silence, prior consent for another exception, inferred intent, a green quality gate, and an ADR written by the agent are not consent. An agent MUST NOT create the consent record on the operator's behalf, mark an issue false-positive or accepted to manufacture a pass, or reinterpret a suppression as classification, cleanup, or temporary CI repair.

---

## 4) Maintainability Guardrails

- Keep one canonical statement of each global rule. Root policy belongs here; scoped files should reference it and add path-specific details instead of repeating or rewording it inconsistently.
- Any change to [the `rv` task registry](./tools/src/revaer_tooling/cli.py), workflow files, release scripts, lint posture, or [`sonar-project.properties`](./sonar-project.properties) must update the relevant instruction file in the same change.
- Review the instruction set whenever crate layout, workflow layout, release flow, or quality gates change materially.
- Keep user-facing docs, examples, and generated API/reference artifacts in sync when exposed surfaces change.

---

## 5) No substantiating artifacts

- Do not create proof reports, evidence bundles, qualification reports, verification documents, audit packages, saved test transcripts, screenshots used as proof, redundant checklists, verification manifests, per-file hash inventories, copied logs, coverage exports, checkpoint files, or duplicate completion records unless Vanna explicitly requests that specific artifact. A request to implement, fix, test, qualify, or complete work is not a request for an evidence package.
- Do not create independent auditors, duplicate verification frameworks, or bespoke evidence generators. Do not add acceptance gates or expand task scope to justify substantiating artifacts.
- Use focused functional regression tests and existing checks. Report results, limitations and relevant commit hashes briefly in chat. Extra paperwork is not an acceptance gate.
- Keep normal command output temporary; do not copy it into the workspace or commit it. Do not copy, package, or retain it as a separate evidence collection. Remove disposable task artifacts when finished without deleting caller-owned work or needed persistent state.
- Do not create an ADR or task record for routine implementation, fixes, testing, cleanup, or instruction edits. Use an ADR only for an architectural decision requiring operator approval or when Vanna explicitly requests one. Keep it concise and use the existing ADR template and indexes only when an ADR is actually warranted.
- Update existing documentation when behavior or an operating contract changes. Do not add documentation, ledger entries, index entries, or status files merely to substantiate completed work.
- Only an explicit user request for a specific substantiating artifact authorizes an exception. A task, skill, old document, previous habit, or agent interpretation of a requirement does not grant permission. This section overrides blanket evidence-retention and per-task record requirements in scoped instructions and supporting documents. Product records used by the application as domain evidence, required contracts, and functional test fixtures must remain intact. Existing functional quality gates remain required.
