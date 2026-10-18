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
> 8. **Qualify locally before pushing**. Section 3 defines targeted-first validation, local Sonar review and one complete local qualification.
> 9. **Dependencies are injected**. Runtime logic receives collaborators from callers; only bootstrap/wiring code constructs concrete implementations or reads the environment.
>
> **Completion Rule:** Final implementation delivery requires the applicable invariants and Section 3 qualification to pass. An operator-requested planning or review checkpoint completes only that stage; report its focused checks and outstanding qualification without starting unrequested implementation or full validation.

---

## 0) Policy Precedence And Source Of Truth

- [`AGENTS.md`](./AGENTS.md) is the non-negotiable root contract.
- Scoped instruction files under [`.github/instructions/`](./.github/instructions/) specialize implementation requirements for their matching paths. They may neither relax root policy nor invent acceptance gates, expand coverage targets or require additional deliverables. Changes to the acceptance contract require operator approval.
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

### Coverage

- Required coverage targets are production code and `rv`, including `tools/src/revaer_tooling/**` and `tools/launcher/src/revaer_launcher/**`. `rv` is the only designated essential internal tool; expanding that designation requires operator approval. Preserve existing production and tooling thresholds, including Rust/Yew WebAssembly, runtime SQL and first-party native requirements.
- Tests, fixtures, harnesses, examples, documentation, unused templates, generated artifacts, vendor code and other internal utilities have no independent coverage percentage, positive-hit or exact-count requirements. This includes `setup.sh`, asset synchronization and documentation utilities. Their applicable functional, lint and security checks remain required.
- Run tests under the supported coverage tools and use their configured reports and exit status to enforce thresholds. Configure production/tooling scope through supported tool options. Do not write custom percentage parsers, subtract estimated lines, fabricate hits or add tests merely to execute test scaffolding. Report tool limitations briefly; do not chase perfect classification of inline test code or build replacement coverage analysis.
- Third-party collectors and analyzers are upstream responsibilities. Select suitable dependencies, consume their supported reports and track relevant upstream issues and fixes. Do not verify collector internals, recreate their algorithms or add synthetic collector conformance gates. Ordinary tests for our invocation, report ingestion and failure propagation remain appropriate.
- Keep complete genuine measurements required by existing functional gates. Their inventory does not expand coverage targets. Sonar coverage-only scope is defined in `sonar-project.properties` and enforced by `tools/src/revaer_tooling/policy/sonar.py`; the separate `rv` gate covers its designated implementation paths. Coverage scope does not narrow first-party analysis or security checks.

### Validation and delivery

- For each fix, pass the smallest relevant check first; for multiple issues, pass each targeted check before related validation. Then run `rv ci-qualify` once on the final candidate before any push. It includes `rv ci`, `rv ui-e2e` and route checks; do not run those components again on an unchanged candidate. Follow the registry and `tasks/quality.py` for composition and prerequisites.
- Complete local Sonar file/project analysis of changed authored code before pushing, using the available Sonar integration with the project's rules. No PR number, published server analysis, or draft push is required for local review. Report supported languages and any unavailable analysis honestly; do not substitute a scan of main, create a PR to unlock analysis, or claim a full hosted gate from local feedback. Fix new local findings and applicable findings in changed production code; distinguish unchanged baseline findings without expanding the task. Raise a material analysis limitation before pushing if it leaves changed production code unchecked.
- Push only after local qualification and local Sonar review succeed. GitHub CI provides independent verification and the authoritative published Sonar quality gate. Required hosted checks and branch protections must pass before merge; local feedback does not replace them.
- Repeat full qualification only after a relevant correction passes targeted checks. Never restart a live run, retry unchanged failures blindly, or combine separate partial successes into a clean qualification.
- An operator-requested planning or review checkpoint ends at that stage: report focused checks and remaining work without starting full qualification or claiming final delivery.
- Use `./setup.sh` for initial bootstrap and the locked `rv` CLI thereafter. Workflows may install tools and stage artifacts; build/test/lint/release gates call `uv run --locked -- rv`. Do not restore legacy Just, Node or shell implementations as gate authorities.

### Sonar and security

- `sonar-project.properties` is the sole source of scanner criteria; its exact allowlist is enforced by `policy/sonar.py`. No workflow overrides, duplicate keys, undocumented properties or alternate scanner criteria are permitted. Hosted PR and main scans retain the same required analyzer and coverage inputs, verified published results and complete nonempty scanner report.
- Keep all authored tracked entries in main-code analysis and security scope; `sonar.tests` points only to the committed empty `.sonar-test-scope` sentinel. Do not populate it, restore automatic test-path classification or misroute languages to hide findings. Coverage exclusions do not remove first-party analysis. No new unresolved issue or `TO_REVIEW` hotspot may remain; historical unresolved issues also block in active first-party production code, including the runtime SQL initializer. Reviewed hotspots do not block.
- Vendor packages/assets and the intentionally malformed YAML fixture follow the exact source exclusions in `policy/sonar.py`. We own dependency selection, security audits, tracking relevant upstream issues and updating when fixes are available; we do not own vendor code quality or patch vendors to satisfy our source gates. First-party wrappers and FFI remain our responsibility.
- Hosted analysis fails on missing required inputs or published metrics, ignored gate conditions, missing SCM blame, incomplete scanner evidence, skipped entitled analysis, analyzer degradation, quality-of-analysis warnings or scanner `WARN` lines. Preserve complete history, configured SCM behavior, native compiler inputs and supported analyzer caches. Fix the owning input or code; do not hide sources or findings.
- Without explicit operator consent naming the exact setting, scope, reason and expiry, do not weaken scanner scope, rules, severity, thresholds, analysis features, dependency analysis, source attribution, coverage/duplication filters, quality profiles/gates, new-code definitions or branch protection. This applies locally and on the server. Do not use suppressions, issue/hotspot dispositions, scanner skips or artifact deletion to manufacture a pass. An agent-written ADR, silence, another exception or a green gate is not consent. Apply only the change actually authorized.

---

## 4) Efficient Work And Maintainability

- Stay on the requested critical path. Follow instruction precedence when unsure, inspect the owning task/configuration and reproduce narrowly. If uncertainty affects correctness or scope, a tool is unavailable, or a targeted attempt yields no new information, report the blocker and smallest decision needed before repeating experiments or expanding validation. Continue independent authorized work; do not invent requirements to resolve uncertainty.
- Keep one canonical statement of each global rule here. Scoped files add implementation details without adding gates, targets or deliverables. Reference the registry/workflows for command composition and configuration for pins. Historical rationale belongs in existing ADRs and reference docs, not active procedures or new ledgers.
- Rust importer adaptation retains measured tracked Rust records, merges same-source compilation counters, preserves uncovered locations and converts unexecuted branch markers to zero hits. Keep complete original LLVM/native reports. Read exact paths and SCM settings from the canonical configuration; do not duplicate repair narratives.
- Changes to the task registry, workflows, release automation, lint posture or Sonar configuration must update the relevant instruction file in the same change. Keep exposed docs/examples/API artifacts aligned with behavior. Review instructions when layouts or operating contracts change materially.
- Write prose as one physical line per paragraph/list item. Keep structural Markdown/YAML/code newlines; do not manually wrap prose.

---

## 5) Measurements And Artifacts

- Measure actual behavior and report relevant results, limitations and commit hashes briefly in chat. Do not create routine task records, ADRs, proof reports, evidence bundles, saved transcripts/log copies, screenshots as proof, checklists, verification manifests, hash inventories, coverage exports, JSON proof/status files or checkpoint ledgers. Implementing, testing or qualifying work does not authorize those artifacts.
- Use existing functional checks and focused regressions. Do not create independent auditors, duplicate verification frameworks, bespoke evidence generators or acceptance gates. Keep ordinary output temporary and remove disposable task artifacts without deleting caller-owned work or needed persistent state.
- Update existing documentation when behavior or contracts change. Create an ADR only for an architectural decision requiring operator approval or when explicitly requested. Do not add docs/index entries merely to substantiate completion.
- Only an explicit request for a specific substantiating artifact authorizes it. Existing functional gates' required outputs, application domain evidence, required contracts and functional fixtures remain intact; their retention is not permission for a separate evidence package. This section overrides blanket retention or per-task-record demands in scoped instructions and supporting docs.
