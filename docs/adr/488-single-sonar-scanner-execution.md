# Single authoritative Sonar scanner execution

- Status: Proposed
- Date: 2026-08-15
- Operator approval: Pending
- Context:
  - ADR 100 selected the official Sonar scan action before the repository required
    every CI operation to run through a canonical `just` recipe and before scanner
    warnings and the complete submitted report became retained, fail-closed
    evidence.
  - The strict-foundation prototype currently preserves that action and then runs
    `just sonar-scan` a second time so the scanner output can be captured, checked
    for every `WARN` line, packaged, and correlated with the published result.
  - Two sequential analyses of the same commit do not add independent coverage or
    analyzer signal. They consume scanner capacity, can publish different task IDs,
    and make the retained report ambiguous about which analysis produced the
    required status.
- Decision:
  - Recommended option: run exactly one signature-verified, exact-version Sonar
    scanner through `just sonar-scan` in both pull-request and main workflows.
  - Provision the scanner in the repository setup action, but make the canonical
    recipe the only scan invocation. The recipe must retain the complete output,
    reject every scanner `WARN`, preserve `.scannerwork`, package the submitted
    report, and expose the one task ID used by the post-scan API verifier.
  - Preserve every accepted input from ADR 100: complete Rust coverage, native
    coverage, the C-family compilation database, full SCM history, and analysis
    of all first-party native sources. This decision changes scanner invocation
    ownership only.
  - Update ADR 100 to record that its official-action invocation is superseded;
    its coverage and C-family decisions remain accepted.
  - Alternative considered: run only the official action. Rejected because its
    step output is not available to a later in-job recipe as a complete retained
    log, so the repository cannot prove that scanner warnings were absent.
  - Alternative considered: keep both scans. Rejected because duplicate analysis
    is slower, produces two task identities, and cannot establish which report is
    authoritative.
- Consequences:
  - Each workflow publishes one Sonar analysis, one scanner log, one report task,
    and one retained submitted report.
  - The repository owns exact scanner installation and signature verification
    instead of delegating invocation to a third-party action.
  - Scanner-version updates require an explicit version, checksum, signature
    verification path, guardrail fixture, and task record.
- Follow-up:
  - Remove the duplicate official scan step from pull-request and main workflows.
  - Make workflow guardrails reject zero or multiple scanner invocations and prove
    that retained evidence and published-result verification consume the same task
    ID.
  - Run the scanner remotely and verify zero `WARN` lines, positive coverage,
    native source records, retained nonempty evidence, and the required quality
    gate result.

## Implementation Boundary

- Approval authorizes only the single-invocation ownership change, the narrow ADR
  100 supersession note, and corresponding workflow, setup, recipe, guardrail,
  instruction, and test changes.
- Approval does not authorize changing scanner scope, exclusions, analyzer
  activation, coverage instrumentation, quality-gate conditions, issue or hotspot
  disposition, server settings, required checks, or any exact input required by
  the accepted strict Sonar policy.

## Task Record

- Motivation:
  - Remove duplicate analysis while preserving the repository's strict retained
    scanner-evidence contract.
- Design notes:
  - The repository setup action remains the installer boundary; `just sonar-scan`
    becomes the sole operational boundary.
- Test coverage summary:
  - Proposal only. Implementation requires scanner-wrapper unit fixtures,
    structured workflow guardrails, and a live remote analysis with one task ID.
- Observability updates:
  - Retain one complete scanner log, one SCM evidence file, one report task, and
    one submitted-report archive per run.
- Status-doc validation:
  - The ADR index and documentation summary expose the proposal. No scanner
    implementation has been changed by this record.
- Risk & rollback plan:
  - If the canonical invocation cannot produce or retain complete evidence, stop
    the rollout. Do not roll back to duplicate scans or accept missing warnings;
    return with exact evidence for another operator decision.
- Dependency rationale:
  - Removes the scan-action runtime dependency. Adds no package dependency; the
    exact Sonar scanner already provisioned by repository setup remains required.
- Stale-policy check:
  - Reviewed `AGENTS.md`, the DevOps and Sonar instructions, ADR 100, both Sonar
    workflows, the setup action, and the scanner wrapper and result guardrails.
  - Drift found: ADR 100's invocation owner conflicts with the current canonical
    `just` and retained-warning requirements.
