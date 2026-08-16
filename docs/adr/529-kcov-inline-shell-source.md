# Fail-closed kcov Bash tracing

- Status: Accepted
- Date: 2026-08-16
- Operator approval: Not applicable: nonarchitectural task record
- Context:
  - Kcov instruments authored Bash with a `PS4` trace expression that reads
    `BASH_SOURCE`.
  - Policy tests invoke selected `just` recipes whose inline `bash -c` bodies
    enable nounset. Those inline shells have no source file, so trace expansion
    failed before the database and exact-tool assertions ran.
  - The same missing identity affects temporary Bash command doubles, while
    assigning one fixed source through the complete process would corrupt
    child-script path discovery and coverage identity.
  - Bash before 4.2 has no `BASH_XTRACEFD`, so kcov multiplexes trace records
    with ordinary standard error and can report parser failures while returning
    success.
  - Kcov derives its trace descriptor from one quarter of the soft open-file
    limit. On macOS, the inherited soft limit can exceed the kernel's per-process
    maximum, making that derived descriptor invalid even with a compatible Bash.
- Decision:
  - Resolve Bash deterministically from an explicit override or known executable
    locations and require both version 4.2 or newer and working
    `BASH_XTRACEFD` behavior.
  - Run kcov and every traced child shell with that Bash first on `PATH`.
  - Cap only kcov's inherited soft open-file limit at 4096, preserving a valid
    trace descriptor without changing application or user-session limits.
  - Validate kcov's generated Bash helper against the one exact pinned prompt
    before child tests run.
  - Replace only `${BASH_SOURCE}` in that prompt with the nounset-safe
    `${BASH_SOURCE:-kcov-inline}` expression and accept the exact patched form
    idempotently.
  - Preserve the pinned kcov binary, file-backed source identity, all authored
    script coverage, nounset, and every coverage threshold.
  - Retain the complete kcov log and fail when kcov exits nonzero, log capture
    fails, its exact version drifts, or the log contains a kcov error or warning.
- Consequences:
  - Kcov can trace inline recipe shells and temporary command doubles without
    failing on an unset source.
  - Trace records and ordinary standard error remain distinct, and an oversized
    host file limit cannot select a descriptor outside the kernel limit.
  - File-backed child scripts continue to resolve their own paths and produce
    distinct coverage records.
  - An unexpected generated-helper shape fails coverage before policy tests run.
- Follow-up:
  - Reconcile the exact helper contract when the pinned kcov revision changes.

## Task Record

- Motivation:
  - Restore fail-closed script coverage so the full local and Sonar coverage
    gates execute the intended database lifecycle and tool-version assertions.
- Design notes:
  - The repair changes only the coverage subprocess environment and generated
    trace helper; recipe behavior, coverage scope, and strict shell options are
    unchanged.
- Test coverage summary:
  - Added supported-interpreter resolution, explicit-invalid-interpreter,
    fallback, exact helper replacement, idempotence, malformed-helper,
    zero-exit diagnostic, nonzero-exit, logging, and kcov-version tests. Focused
    policy-suite, script-coverage, and full local validation gates are required
    before handoff.
- Observability updates:
  - `coverage/scripts/kcov.log` retains the complete coverage-process output and
    records the selected Bash version and bounded file limit.
- Status-doc validation:
  - Product status and operator documentation do not change. The DevOps
    instruction, ADR index, and documentation summary record the harness rule.
- Risk & rollback plan:
  - A kcov revision that changes the helper prompt blocks coverage until the
    contract is reviewed. Roll back the generated-helper preparation only after
    replacing kcov's source-identity contract with an equally strict one.
- Dependency rationale:
  - No repository dependency is added. Bash 4.2 or newer is required only for
    script coverage; supported CI images already provide it and macOS developers
    can supply it through the existing Homebrew toolchain.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/devops.instructions.md`,
    `just/quality.just`, `just/database.just`, and the policy-suite scripts.
  - Drift was found between the pinned kcov trace behavior, nounset inline
    recipes, and the unbounded host file limit; the DevOps instruction and
    executable harness now agree.
