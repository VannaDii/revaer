# Exact Cargo-installed tool path resolution

- Status: Recorded
- Date: 2026-08-16
- Operator approval: Not applicable: nonarchitectural task record
- Context:
  - `scripts/ensure-exact-cargo-tool.sh` discovered tools through `PATH`, while
    Cargo installs them under `${CARGO_HOME:-$HOME/.cargo}/bin` and canonical
    documentation recipes execute that exact location.
  - A Homebrew `mdbook` 0.5.4 earlier on `PATH` therefore caused the helper to
    replace an already-correct Cargo-home `mdbook` 0.5.0.
  - Cargo external subcommands require the subcommand name in their executable
    argument shape, so probing a `cargo-*` executable directly still must model
    Cargo's invocation contract.
  - Ambient `CARGO_INSTALL_ROOT` or Cargo configuration could otherwise direct
    an install away from the executable the helper verifies.
  - Cargo can successfully install the exact binary while emitting a rustup
    PATH warning. Treating strict warning status 65 as retryable repeatedly
    reinstalled the binary even though the warning could not change on retry.
- Decision:
  - Resolve the owned Cargo home from `CARGO_HOME`, falling back to
    `$HOME/.cargo`, and inspect only its exact `bin/<binary>` executable.
  - Pass that Cargo home through Cargo's explicit `--root` option and reject a
    caller-supplied install-root override so installation and verification
    cannot diverge.
  - Probe ordinary binaries as `<exact-path> --version`. Probe `cargo-*`
    binaries as `<exact-path> <subcommand> --version`, preserving Cargo's
    external-subcommand argument contract without PATH lookup.
  - Report the exact executable path for retained, replacement, successful
    post-install, and failed post-install checks.
  - Preserve every reviewed version pin, `--locked`, `--force`, bounded retry
    for genuine Cargo failures, warning detector, and warning-failure status.
  - Keep warning-marked success fail-closed with status 65, but classify it as
    non-retryable after the first successful install. Post-probe the exact
    executable for evidence and preserve the installer status. Continue to
    retry genuine nonzero Cargo exits and preserve their terminal status.
  - Keep one policy-covered regression harness; the historical script entry
    point delegates to it so the two entry points cannot drift.
- Consequences:
  - An unrelated same-named executable earlier on `PATH` cannot cause a false
    version result or unnecessary reinstall.
  - Cargo configuration cannot silently place a replacement outside the path
    canonical recipes consume.
  - Missing Cargo-home configuration and caller install-root overrides now fail
    with usage status instead of falling back to an ambiguous executable.
  - Tool checks provide actionable path evidence in local and CI logs.
  - A warning-producing successful install fails its current gate once without
    entering a reinstall loop; the next invocation accepts the exact installed
    binary if its version matches.
- Follow-up:
  - Keep the regression cases aligned with Cargo's external-subcommand argument
    contract when adding another exact Cargo-installed tool.

## Task Record

- Motivation:
  - Stop deterministic documentation setup from reinstalling pinned mdBook when
    a newer Homebrew binary shadows the valid Cargo-home executable.
- Design notes:
  - The repair changes only tool location resolution, version probing, the
    explicit Cargo install root, and retry classification for a successful
    process whose warning output is converted to status 65. Warning detection,
    status 65, retry limits for genuine Cargo failures, tool versions, and
    downstream quality commands remain strict and unchanged.
- Test coverage summary:
  - The focused harness covers missing, older, exact, newer, and bad
    post-install versions for a Cargo subcommand; wrong direct and subcommand
    binaries earlier on `PATH`; exact-match no-reinstall behavior; Cargo-home
    fallback; leading `v` parsing; explicit-root ownership; and missing-home
    failure.
  - A warning-producing successful install is exercised with three attempts
    configured: it installs once, returns exact status 65, reports the observed
    Cargo-home binary, and the next invocation performs no reinstall. A genuine
    Cargo failure consumes all three bounded attempts and preserves terminal
    status 42.
  - Both focused entry points passed. Real mdBook 0.5.0 and cargo-llvm-cov 0.8.7
    probes reported their exact Cargo-home executables without reinstalling.
  - `just ci` passed in full, including policy, lint, dependency checks, audits,
    all test matrices, per-crate Rust coverage thresholds, shell coverage, and
    the optimized release build. `just instruction-drift`, `just docs`,
    `just docs-link-check` (943 of 943 links), `just check`, Bash syntax checks,
    and `git diff --check` also passed independently.
  - `just ui-e2e` reached Playwright after its dependency audit and generated
    client checks, then failed because the unchanged parent file
    `tests/playwright.config.ts` references undefined `envDir` at line 53. This
    inherited failure is outside the exact Cargo-tool-path repair.
- Observability updates:
  - Version-check output now includes the exact executable path for success,
    replacement, installation, and post-install failure.
- Status-doc validation:
  - Product status does not change. DevOps operator behavior changes only by
    stopping retries after a warning-marked successful install. The DevOps
    instruction, ADR index, and generated documentation summary record the
    corrected tooling contract.
- Risk & rollback plan:
  - A Cargo tool with a nonstandard external-subcommand argument shape will
    fail its post-install check instead of being accepted ambiguously. Rollback
    is the single commit, but would restore PATH-shadowed verification and is
    not appropriate without an equivalent exact-path replacement.
- Dependency rationale:
  - No dependency or tool version is added or changed.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/devops.instructions.md`,
    `just/quality.just`, `just/docs.just`, `scripts/cargo-install-retry.sh`, and
    both exact-tool test entry points.
  - Drift was found between PATH-based verification and Cargo-home execution;
    the helper, scoped instruction, and focused regression harness now agree.
