# Uniform Stacked-PR Check Emission

- Status: Accepted
- Date: 2026-08-15
- Operator approval: Option A approved wholesale by the operator on 2026-08-15: "I approve the ADRs as they are now and I'm resuming your goal."
- Context:
  - The active `Stacks` ruleset targets `refs/heads/stack/**/*`, requires 21 status contexts, and intentionally omits `CodeQL`, `code_scanning`, and `code_quality` requirements that remain on `Default Branch`.
  - A ruleset can require a status context, but it cannot cause a workflow to create that context.
  - All 99 current leaf-ancestor branches define real `Run Audit`, `Check Deny`, and `Check Unused Deps` jobs, but none emits the required `Supply Chain Checks` context.
  - `Media Conversion Fixtures` is defined on only 23 of those 99 branches and first persists at `stack/media3-60b-media-fixture-cleanup`.
  - The existing fixture catalogue provides locked source acquisition, real FFmpeg-derived fixture generation, probe verification, integrity tests, and cleanup machinery that can establish a meaningful gate before the application runtime exists.
- Decision:
  - Select Option A: create a dedicated validation-foundation pull request beneath the current bottom pull request and move the fixture catalogue and uniform check contract into it.
  - Add `Media Conversion Fixtures` at that first boundary. It must download only locked inputs, perform real FFmpeg fixture generation, verify reviewed probe output and integrity constraints, publish a nonempty report, and clean generated media under `if: always()`.
  - Strengthen the same canonical `just` recipe when the application runtime is introduced so it additionally runs production-adapter conversion tests. The gate may become stricter but may never become a no-op or conceal failed verification.
  - Add `Supply Chain Checks` with `needs: [audit, deny, udeps]` and `if: always()`. A canonical `just`-backed verifier must fail unless all three upstream jobs completed successfully; failure, cancellation, skip, empty, and unknown results all fail.
  - Add structural guardrails for the exact names, dependencies, fail-closed result handling, report publication, and media cleanup.
  - Move the current supply-vendor delivery to second position, replay every descendant, remove the displaced fixture-catalogue change from its former position, and preserve the intended final tree apart from the approved workflow contract.
- Consequences:
  - Every pull request receives real media-fixture and supply-chain results without changing the approved ruleset.
  - The fixture gate validates actual generation and inspection first, then adds service-level execution as that capability enters the stack.
  - The existing bottom pull request is already approximately 9,748 changed lines. The approximately 2,900-line fixture catalogue therefore belongs in a separate first pull request to preserve the 10,000-change limit.
  - All descendants must be replayed and rerun on their final head/base pairs.
  - The CodeQL exception applies only to bases governed by `Stacks`; the bottom pull request targeting `main` remains subject to `Default Branch` CodeQL requirements.
  - No skipped required check, synthetic success, admin merge bypass, or Sonar, coverage, security, review, or cleanup relaxation is authorized.
- Follow-up:
  - Implement the workflow contract in an isolated worktree and update the Justfile, guardrails, tests, and scoped instructions together.
  - Run the fixture gate, guardrail tests, `just ci`, and `just ui-e2e` at the validation boundary and complete-stack leaf, cleaning generated media after every run.
  - Replay with recorded object IDs and exact force-with-lease updates only after local validation.
  - Confirm through the GitHub API that every open stack pull request emits and passes every context required by its effective base ruleset.

## Implementation Boundary

- This decision records the operator-applied ruleset state but does not authorize another ruleset mutation.
- Approval authorizes the fixture-catalogue reorder, the two missing workflow contexts, their canonical recipes and guardrails, required instruction and ADR updates, and the minimum descendant replay needed to preserve one linear stack.
- Approval does not authorize merging, bypassing required checks, adding `CodeQL` to `Stacks`, or changing `Default Branch`.
- If the first delivery cannot run a meaningful fixture conversion gate, remain below 10,000 changes, or preserve the final tree, implementation must stop rather than substitute a passing placeholder.

## Task Record

- Motivation:
  - Make the approved stack rules enforce checks that workflows actually and uniformly emit at every independently reviewable boundary.
- Design notes:
  - The decision follows outside-in delivery by moving executable fixture contracts ahead of runtime implementation.
  - `pr.yml` remains the sole pull-request validation workflow.
  - The supply aggregate reports the three dependency-gate outcomes without replacing them.
- Test coverage summary:
  - Live API inspection confirmed 21 required stack contexts with `CodeQL` excluded.
  - An exact definition audit found `Media Conversion Fixtures` on 23 of 99 branches and `Supply Chain Checks` on none.
  - PR 142 proves that a job added to the existing `pr.yml` is emitted on the pull request that introduces it.
- Observability updates:
  - Retain the media conversion report, Sonar evidence, supply results, and a machine-readable final audit of required contexts per pull request.
- Status-doc validation:
  - The ADR catalogue exposes this accepted decision without claiming the runtime program is complete.
- Risk & rollback plan:
  - Validate the first boundary before replaying descendants and retain exact pre-replay object IDs.
  - Restore recorded object IDs with exact leases if the approved replay regresses; do not alter unrelated branches or rulesets.
- Dependency rationale:
  - No package dependency is added. The initial gate uses existing pinned actions, system FFmpeg tools, fixture scripts, and `just` recipes.
- Stale-policy check:
  - Reviewed `AGENTS.md`, the DevOps and Rust instructions, `pr.yml`, `build-images.yml`, `justfile`, and live rulesets.
  - Drift was found between required contexts and workflow definitions; no quality criterion is relaxed.
