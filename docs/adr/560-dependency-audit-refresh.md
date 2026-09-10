# Dependency audit refresh and policy precedence

- Status: Recorded
- Date: 2026-09-09
- Operator approval: Not applicable: nonarchitectural task record
- Context:
  - The fresh validation recorded in ADR 559 stopped at Rust and JavaScript
    dependency audits. Those failures prevented the later tests and coverage
    from establishing current feature evidence.
  - The scoped Rust instruction incorrectly claimed precedence over the root
    contract, contradicting `AGENTS.md` section 0.
- Decision:
  - Update only existing dependency versions needed to remove reported audit
    findings. Preserve all security severities, empty exception lists, exact
    analysis-tool versions, coverage requirements, and release gates.
  - Make the scoped Rust precedence sentence agree with the existing root rule.
    This corrects contradictory guidance; it grants no architectural discretion.
- Consequences:
  - Passing audits permit later gates to run but do not establish media-runtime
    readiness, supported package behavior, or successful end-to-end workflows.
  - Dependency fixes must reach every affected stack revision, not only the
    local integrated checkout.
- Follow-up:
  - Run `just ci` and `just ui-e2e` against the combined approved implementation.
    Record failures without suppressing them or treating an audit pass as a
    complete gate pass.
  - Carry these fixes through the linear stack and revalidate exact PR heads.

## Task Record

- Motivation:
  - Unblock real validation after dependency disclosures since the saved
    integration revision, without relaxing any criterion.
- Design notes:
  - Rust `h2` moves from `0.4.12` to `0.4.16`, the minimum fixed version in
    `RUSTSEC-2026-0258` for unbounded empty HTTP/2 DATA-frame queuing.
  - Rust `chacha20` moves from yanked `0.10.0` to published patch `0.10.2`.
    Cargo also reconciles existing Windows-only dependency edges to versions
    already in the lock. No new crate, dependency exception, or direct Rust
    dependency is introduced.
  - The tests graph updates existing exact overrides for `fast-uri` from `3.1.5`
    to `3.1.6` and `js-yaml` from `4.3.1` to `4.3.2`. The release graph updates
    only transitive `js-yaml` to `4.3.2`. No Redocly or other parent upgrade is
    needed. Existing NVM-aware recipes remain authoritative.
- Test coverage summary:
  - `just audit`: passed on the updated Rust lock, including a clean repeat
    without the first run's transient advisory-database lock warning.
  - `just deny`: passed; advisories, bans, licenses, and sources all passed.
  - `just instruction-drift`: passed after the scoped precedence correction.
  - The JavaScript worker passed `just api-test-client`,
    `just js-release-coverage`, and commit-scoped `just instruction-drift`.
    Tests audit findings fell from four high-severity entries to zero; release
    audit findings fell from one high-severity entry to zero. Supplemental
    Node assertions exercised valid URI/schema resolution and YAML merges.
  - Full CI and UI validation remain pending in this task record until the
    integrated run finishes; no full-gate success is inferred here.
- Observability updates:
  - No runtime telemetry changes. Retain complete non-media gate logs outside
    disposable worktrees for review and follow-up.
- Status-doc validation:
  - The first-release scope in `MEDIA_TRANSCODING.md` and ADR 559 remains intact.
    No product guide or readiness claim changes from a dependency patch.
- Risk & rollback plan:
  - Patch updates can change dependency behavior. Required workspace, native,
    feature, and UI gates remain mandatory before merging.
  - A regression requires another patched compatible resolution or a blocked
    release, not restoration of a vulnerable/yanked dependency as an acceptable
    final state. Preserve the prior lock in Git for diagnosis.
- Dependency rationale:
  - Reuse the existing packages and semver-compatible patch lines. No new
    dependency or generalized update is needed for the reported findings.
- Stale-policy check:
  - Reviewed `AGENTS.md`, Rust/devops scoped instructions, the ADR template,
    `just/quality.just`, `just/ui.just`, and the current lock/manifests.
  - Removed the scoped-over-root precedence contradiction. No workflow, Sonar
    setting, source scope, required check, or quality threshold is weakened.
