# AGENT.MD — Codex Operating Instructions (Revaer, Rust 2024)

> **Prime Directives**
>
> 1. **Rust 2024 only**. Never lower the edition.
> 2. **No dead code**. No unused items, no future stubs, no parking-lot code.
> 3. **Minimal dependencies**. Prefer `std`; every new dependency needs written rationale.
> 4. **The Justfile is law**. Local and CI build/test/lint/release gates run through [`justfile`](./justfile).
> 5. **Stored procedures or bust**. Runtime database access goes through stored procedures; raw SQL belongs only in migrations and tightly scoped operational bootstrap scripts.
> 6. **Deterministic, panic-free production code**. No `panic!`, `unwrap()`, `expect()`, `unreachable!()`, or silent error suppression in authored production or bootstrap code.
> 7. **No source-level lint suppressions**. `#[allow(...)]` and `#[expect(...)]` are not permitted in authored code.
> 8. **`just ci` and `just ui-e2e` before every hand-off**. A task is not complete until both pass cleanly.
> 9. **Dependencies are injected**. Runtime logic receives collaborators from callers; only bootstrap/wiring code constructs concrete implementations or reads the environment.
>
> **Completion Rule:** Because Codex runs locally, a task is complete only when all requirements in this file and the scoped instruction files are satisfied, `just ci` passes without warnings or errors, and `just ui-e2e` passes.

---

## 0) Policy Precedence And Source Of Truth

- [`AGENTS.md`](./AGENTS.md) is the non-negotiable root contract.
- Scoped instruction files under [`.github/instructions/`](./.github/instructions/) may only tighten or specialize the root contract for their matching paths. They may not relax root policy.
- If two instruction files appear to conflict, precedence is:
  1. [`AGENTS.md`](./AGENTS.md)
  2. the most specific scoped instruction file
  3. supporting docs and ADRs
- Operational source-of-truth files are:
  - [`justfile`](./justfile)
  - [`.github/workflows/ci.yml`](./.github/workflows/ci.yml)
  - [`.github/workflows/pr.yml`](./.github/workflows/pr.yml)
  - [`.github/workflows/sonar.yml`](./.github/workflows/sonar.yml)
  - [`sonar-project.properties`](./sonar-project.properties)
- This file must reference those operational files instead of copying large command bodies or stale workflow inventories.
- Current scoped instruction files:
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
- Advisory ignores are forbidden by default: [`.secignore`](./.secignore), the advisory-ignore list in [`deny.toml`](./deny.toml), and npm audit exceptions must remain empty. Cargo and npm findings at every configured severity are mandatory fixes. Any exception requires explicit operator consent recorded in the task ADR, an expiry, and a guardrail change in the same commit. Exact duplicate-crate tolerances in `deny.toml` must be ADR-backed, time-bounded, and removed as soon as the graph can converge.

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
- If a rule cannot be satisfied cleanly, redesign, split, delete, or isolate the code behind the documented FFI boundary. Do not silence the rule.

---

## 3) Quality Gates

- All local and CI operations run through `just` recipes. Workflows may install tools or stage artifacts, but build/test/lint/release gates must call `just`.
- Handoff requires:
  - `just ci`
  - `just ui-e2e`
- [`justfile`](./justfile) is the canonical command surface for build, lint, test, coverage, release, docs, and local dev loops.
- [`ci.yml`](./.github/workflows/ci.yml), [`pr.yml`](./.github/workflows/pr.yml), and [`sonar.yml`](./.github/workflows/sonar.yml) must stay aligned with the Justfile and this policy.
- Sonar analysis scope and first-party signal shaping are versioned in [`sonar-project.properties`](./sonar-project.properties). `sonar.sources` must enumerate every authored tracked top-level repository entry so all authored content remains in main-code scope without treating the untracked `.git` object database as source. `sonar.tests` must point only to the committed empty `.sonar-test-scope` sentinel: this disables Sonar's path heuristic, prevents `docs` and `tests` paths from silently skipping secrets analysis, and keeps every authored test under main-code rules and coverage. Strict empty scope filters are intentional scanner-side overrides of any project-side narrowing. Rust coverage must exercise the complete workspace with all features, produce real LCOV source and line records, and publish positive coverage and lines-to-cover metrics.
- Sonar strictness is mandatory and fail-closed. Do not add, broaden, or reintroduce Sonar issue ignores, rule-scope restrictions, source/test inclusions or exclusions, coverage exclusions, duplication exclusions, analyzer default exclusions, binary suffix exclusions, bundle/generated-code skips, dependency-analysis degradation, disabled available analyzers, disabled or unavailable analyzer caches that reduce cross-file analysis, partial SCM reload, discarded scanner evidence, file-size lowering, SCM disablement, SCM-ignore-based hiding, scanner skip switches, quality-gate relaxation, `NOSONAR`, or comparable suppressions without explicit operator consent recorded in the task ADR. `sonar-project.properties` is the only permitted scanner-criteria source: workflow overrides, duplicate keys, and unreviewed properties are forbidden, and every accepted property must remain on the exact allowlist enforced by `scripts/workflow-guardrails.sh`. Do not classify any authored first-party file as test-only or out of the source gate to make findings disappear, alter or populate `.sonar-test-scope`, restore automatic test-path detection, route files into the wrong language analyzer, or remove coverage instrumentation to manufacture a cleaner log. The post-scan gate must prove that positive coverage and lines-to-cover metrics were published, the quality gate evaluated without ignored conditions, no new unresolved issue or unreviewed hotspot remains, and the complete submitted scanner report was retained as nonempty evidence. Do not use vendored/generated discomfort, binary assets, scanner noise, time pressure, coverage discomfort, dependency-resolution failures, or a desire to make CI pass as an excuse to relax Sonar. A failing Sonar gate, skipped scan, skipped entitled dependency analysis, missing coverage input or metric, missing native analyzer input, disabled analyzer feature flag, unavailable analyzer cache, missing SCM blame, quality-of-analysis warning, deprecated analysis parameter, absent scanner evidence, or scanner `WARN` line is a required fix, not permission to hide files or soften criteria. Fix the finding, add real coverage, supply the precise analyzer input, delete or regenerate the offending artifact, or stop and obtain explicit operator consent before weakening the gate. This prohibition applies equally to repository files and Sonar server state: agents MUST NOT weaken quality-profile rule activation or severity, quality-gate conditions or thresholds, the new-code definition, project or organization analysis settings, issue or hotspot dispositions, or branch-protection requirements as a substitute for fixing code. Agents MUST stop before making any criteria-relaxing edit or remote mutation and ask the operator for consent that names the exact property or server setting, scope, reason, and expiry. Silence, prior consent for another exception, inferred intent, a green quality gate, and an ADR written by the agent are not consent. An agent MUST NOT create the consent record on the operator's behalf, mark an issue false-positive or accepted to manufacture a pass, or reinterpret a suppression as classification, cleanup, or temporary CI repair.

---

## 4) Maintainability Guardrails

- Keep one canonical statement of each global rule. Root policy belongs here; scoped files should reference it and add path-specific details instead of repeating or rewording it inconsistently.
- Any change to [`justfile`](./justfile), workflow files, release scripts, lint posture, or [`sonar-project.properties`](./sonar-project.properties) must update the relevant instruction file in the same change.
- Review the instruction set whenever crate layout, workflow layout, release flow, or quality gates change materially.
- Keep user-facing docs, examples, and generated API/reference artifacts in sync when exposed surfaces change.

---

## 5) Task Record And ADR Rules

- Every task persists a task record alongside the change as an ADR under [`docs/adr/`](./docs/adr/).
- Start from [`docs/adr/template.md`](./docs/adr/template.md), number sequentially, and keep the file name concise and searchable.
- Agents are not authorized to make architectural decisions without explicit operator review and approval. When work exposes a question that requires architectural oversight, the agent must:
  1. stop implementation of the affected decision;
  2. write a `Proposed` ADR that states the problem, constraints, viable alternatives, recommendation, consequences, and implementation boundary;
  3. present the ADR to the operator for review; and
  4. wait for explicit, decision-specific operator approval before changing the ADR to `Accepted` or implementing the decision.
- Silence, inferred intent, approval of a different ADR, implementation progress, passing checks, and an agent-authored recommendation are not architectural approval. Agents must not record approval on the operator's behalf.
- Investigation, reproduction, and evidence gathering may continue only when they do not commit the repository to an architectural choice. If an architectural implementation was prepared before approval, it must remain local and unpushed, the ADR must remain `Proposed`, and affected work must stop until the operator directs whether to accept, revise, or discard it.
- Every ADR must include an `Operator approval` field. `Proposed` ADRs use `Pending`. An ADR may use `Accepted` only when that field records the operator's explicit approval and date.
- Every task record must include:
  - Motivation
  - Design notes
  - Test coverage summary
  - Observability updates
  - Risk and rollback plan
  - Dependency rationale
  - Stale-policy check
- The stale-policy check must record:
  - which instruction files were reviewed
  - whether drift was found
  - which contradictions or stale references were removed
- Update [`docs/adr/index.md`](./docs/adr/index.md) and [`docs/SUMMARY.md`](./docs/SUMMARY.md) in the same change that adds the ADR.
