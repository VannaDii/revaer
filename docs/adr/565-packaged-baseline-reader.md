# Packaged baseline reader

- Status: Recorded
- Date: 2026-09-10
- Operator approval: Not applicable: nonarchitectural task record
- Context: Accepted ADR 551 requires a bounded stored-procedure baseline read
  before the packaged runtime can replace migration-based startup.
- Decision: Implement the read projection and exact value verification without
  changing startup, creating a pool, executing init, or adopting a database.
- Consequences: A verified row proves only its checked contract, digest and
  runtime identity. The owning lifecycle still must prove connection identity,
  server settings, privileges, structural integrity, deadlines and cleanup.
- Follow-up: Integrate this reader with the approved lifecycle after the final
  SQL, pristine classifier and coordinated cutover are validated.

## Task Record

- Motivation: Advance the single-init cutover prerequisite without weakening
  the frozen corpus or treating absent baseline state as a successful result.
- Design notes: Call only `revaer_system.read_database_baseline_v1`, with a
  two-row limit to detect duplicate results. Require one row, contract 1,
  PostgreSQL 160014, a 32-byte exact digest, valid distinct owner/runtime names,
  and an exact runtime-login match. Role validation counts UTF-8 bytes and does
  not trim, lowercase or quote-transform the operator-provisioned identity.
- Test coverage summary: Added deterministic projection, version, digest-byte,
  role-byte, login-substitution, cardinality, SQLSTATE, and error-redaction
  tests. A real disposable PostgreSQL test rejects an unmanaged database and
  malformed caller expectation before test-only fixture DDL. It then rejects a
  deliberately malformed stored-procedure projection with integer role columns;
  production code never performs DDL. Validation results are recorded
  below; pure row tests are not a successful packaged initialization proof.
- Observability updates: Emit a closed ADR 551 reason and validated five-byte
  SQLSTATE at the failure origin, except SQLSTATE `57014`, whose final logging
  is deferred to the lifecycle that owns the cause. Do not retain raw SQLx/PostgreSQL errors as
  an error source. Query cancellation and statement timeout share SQLSTATE;
  their cause remains the owning lifecycle's responsibility, not a guess from
  untrusted message text. The verified value contains no role names.
- Status-doc validation: The normal service still uses the frozen migrations;
  no new CLI mode or runtime pool gate is advertised as operational. The
  separate stack reconstruction encountered the older unfenced claim-v3 ABI
  in PR 122; it was aborted without altering the canonical frozen corpus or
  publishing the historical cutover.
- Risk & rollback plan: This is an additive read-only library boundary. Revert
  it before cutover if its ABI disagrees with the approved SQL; never fall back
  to migration execution or unmanaged-database adoption on a read failure.
- Dependency rationale: No dependency or feature is added. Use the existing
  SQLx projection, chrono timestamp, tracing, and disposable test database helper.
- Stale-policy check: Reviewed `AGENTS.md`, Rust/data/devops scoped rules,
  ADRs 541, 551, 557-559, and the database/quality recipes. The new focused recipe
  and devops obligation do not replace full CI/UI or change a quality criterion.

## Validation

- `just test-database-baseline-read`: all 14 tests passed against a private
  PostgreSQL 16.14 service using ADR 551's exact digest and checksum/locale
  settings. The task-owned service and database were removed after testing.
- Read-only peer review found two issues before commit: role-to-text casts
  could conceal wrong projection types, and eager cancellation logging could
  misclassify or duplicate the lifecycle's final diagnostic. Removed the casts
  and added real wrong-type transport plus captured-event regressions. No
  dependency or production stored-procedure change was needed.
- `just fmt`, `just lint` including both strict workspace Clippy passes and
  policy tests, and `just instruction-drift` passed. The initial lint failures
  were corrected in code and documentation without suppressions.
- `just docs-index` regenerated the documentation catalog. Full integrated
  CI/UI, Linux package evidence and published Sonar remain separate gates.
- `just docs-link-check`: 1,046 links passed, zero errors. Full-file Sonar MCP
  `analyze_code_snippet` ran on all five new Rust files and the negative SQL
  fixture with project `VannaDii_Revaer`, language `secrets`, scope `MAIN`:
  zero reported issues. The MCP does not provide Rust or PostgreSQL semantic
  analyzers; this is secrets guidance, not semantic or published gate proof.
- SQL shape/security proof, successful real sealed-row transport, full packaged
  verification, and runtime cutover are not established by this initial slice.

### Integrated Review

- The first full workspace test run exposed a captured-event assertion that
  passed in the package-focused run but observed zero events in full CI. An
  explicit-interest/maximum-level change did not resolve it. Matching CI's
  workspace-wide feature selection while filtering baseline tests still passed
  all 14, so package feature selection did not explain the failure.
- The captured-event assertion now runs in an exact child test of the same
  binary, following the existing bootstrap-test isolation pattern. The child
  inherits coverage instrumentation and keeps both the zero-event cancellation
  and one-event failure assertions. No workspace test is disabled or serialized,
  and production logging and quality criteria are unchanged. The focused recipe
  retains workspace-wide feature selection to avoid future evidence drift.
- `just test-database-baseline-read` and the full workspace tests within the
  integrated CI rerun passed, including the isolated child. Full-file Sonar MCP
  secrets analysis of the changed logging test returned zero issues; Rust
  semantic analysis remains unavailable through that MCP. `just ci` subsequently
  exited zero for executable revision `fe227810`, including all 18 package
  coverage gates, script coverage, and the release build. Rust LCOV has 232
  source records, 100,731 line records, and 93,644 covered lines. The native
  singleton-directory regression on a separate worker branch was not included
  in this run; later documentation-only updates do not change the tested code.
- The separate media-profile and torrent-authoring E2E failures are not
  addressed by this test-only change.
