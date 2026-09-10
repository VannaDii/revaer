# Media decision review and approval delta

- Status: Proposed
- Date: 2026-09-09
- Operator approval: Pending

## Context

The operator requested reconciliation of ADRs 557 and 558, an exact approval
delta, and a replacement goal statement. That request approves preparation of
these documents, not their recommendations, production implementation, changes
to quality criteria, or activation of a new goal. The existing goal is paused
and is not changed by this record.

The comparison baseline is ADR 557 at `bfb68bf` and ADR 558 at `fb0f6eb`.
The review branch starts from integration commit `7fd5df30`, including accepted
ADRs 523, 550, 551, and 554. Existing source branches and the conflicted primary
checkout remain intact. This is a documentation-only integration, not the
integration of the separate trusted root-source implementation.

## Decision Delta

| ID | August draft | Revised recommendation | Effect of accepting |
| --- | --- | --- | --- |
| R1 | One global catalog generation, with explicit rebinding; unrelated-change disruption was buried in the detailed contract. | Retain that model and explicitly accept that an unused-slot addition or committed source unavailability makes prior bindings stale. | A simpler single-generation authority model; operators must rebind affected profiles/associations and use the approved job re-plan/recovery workflow. No silent revival or per-slot continuity is introduced. |
| R2 | Nonempty association prefix required; a whole library root could not be selected. | Permit explicit empty association prefix for the whole attested root; keep candidate paths nonempty, relative, and descriptor-confined. | Whole-library discovery is expressible without a fabricated child directory. Empty is not a default, null, working directory, or permission to escape the root. |
| R3 | Existing API-key authentication grants read-only root administration with full paths. | Retain it and expose the permission consequence for approval. | Every principal admitted by the existing authenticated API boundary can inspect root paths. No separate administrator role is invented. |
| R4 | 128 active discovery associations per profile version. | Retain it as an explicit capacity choice, not a benchmark result. | The 129th active association is rejected; changing the bound requires approval and validation. |
| B1 | Every initial broker failure prevents API startup. | Distinguish safely settled initial unavailability from fatal lifecycle/containment failure, as detailed in ADR 558. | Remediation can remain available only with both media lanes closed and cleanup proved. Startup timeout, unproven containment/cleanup, and failed recovery remain fatal. |
| B2 | Broker-binary identity and native-tool closure identity had detailed but unimplemented representations. | Retain file length plus SHA-256 for the same-binary handshake, and require ADR 519's complete closure identity for native requests. | The broker is not production-usable until the full closure verifier and immutable job binding exist; a path or executable-only digest is not a fallback. |
| B3 | Floor rounding, identifier exhaustion, one recovery attempt, and several incompatible descriptions of cleanup/startup timing were bundled together. | Explicitly decide deadline origins, identifier retirement, the whole recovery-incident budget, and outcome/exit semantics in ADR 558. | Queue and cleanup cannot silently extend execution authority. The proposed aggregate recovery interpretation is an operator decision, not a private implementation refinement or an already-approved timing claim. |
| E1 | Fixed package environment values and digest appeared to be ready for adoption. | Keep the nine-key isolation policy, but retain the exact table as an unvalidated candidate under a separate evidence/approval hold. | No production environment is selected by approval of B1. Package evidence and explicit approval must resolve exact values and the HOME ownership conflict. |
| G1 | Private implementation names and previously unassigned private codec bytes were bundled with architectural decisions. | Permit only semantics-preserving internal refinements within an approved contract, with reviewed code, updated appendix, and tests; preserve every accepted exact constraint. | Ordinary internal naming or unassigned private codec choices need not create another ADR, but only after the operator explicitly accepts this narrow distinction. |

The complete proposed root contract remains in
[ADR 557](557-root-persistence-contract.md), including normalized persistence,
atomic stored-procedure reconciliation, immutable versions, path visibility,
conditional API mutation, YAML, and the coordinated init-only cutover. The
broker contract and its additional decision questions remain in
[ADR 558](558-rvb1-native-process-broker-wire-contract.md). This table is the
change summary, not a substitute that silently drops constraints in either ADR.

The root appendix also aligns overlap serialization with its existing shared
source scope: the proposed advisory lock is keyed by source attestation rather
than one profile version. API and YAML binding mutations now share one
catalog/source/parent lock order rather than invert source and parent locks.
These corrections and their cross-profile/concurrent tests remain
part of R1's pending persistence contract. No runtime race was reproduced or
fixed by this documentation change.

## G1: Architecture Versus Implementation Detail

### Options

1. Keep every proposed internal name and previously unassigned wire byte
   decision-specific. This is precise but makes semantics-preserving
   implementation refinements repeatedly approval-blocking.
2. Delegate a narrow class of internal refinements while retaining architectural
   and exact accepted-contract approval. This is the recommendation below.
3. Give agents general architectural discretion. Reject this: it conflicts with
   the operator's explicit rule and is not requested.

### Proposed Boundary

- Examples of internal detail: private Rust symbol names, helper decomposition
  within existing ownership boundaries, and numeric tags for a private unshipped
  protocol field that no Accepted ADR has already assigned. Keep concrete
  appendices, codec vectors, schema callers, and tests synchronized in the same
  reviewed change. Do not introduce a new owner, dependency, or abstraction by
  labeling it internal.
- Always require decision-specific architectural approval for scope, observable
  behavior, filesystem authority, data model or retention semantics, privileges,
  public API/YAML contracts, externally consumed identifiers, compatibility,
  process topology, failure classification, queue/admission policy, concurrency,
  deadlines, retry policy, resource bounds, environment policy or exact package
  values under E1, containment, supported platforms, and new dependencies.
- Any value, field, state, constant, layout, or invariant explicitly fixed by an
  Accepted ADR remains fixed. G1 does not retroactively release ADR 554's exact
  constraints or those of any other accepted decision. A proposed change to
  one must name the predecessor constraint, scope, consequence, and requested
  supersession before implementation.
- Changing Sonar scope, analyzers, coverage, thresholds, issue dispositions,
  security exceptions, required checks, or any quality criterion is never an
  internal refinement. Existing operator-consent and expiry requirements remain
  unchanged. No instruction file, workflow, ruleset, or Sonar setting changes in
  this task.
- Ambiguous changes remain on the architectural side of the boundary. The agent
  writes and presents a Proposed ADR and pauses only the affected work; unrelated
  already-authorized work can continue.
- G1 is pending. Until accepted, existing approval requirements apply without
  modification. Agreement to prepare this reconciliation is not approval of G1.

## Remaining Holds

Approving these revisions would permit work within their accepted boundaries,
not release all previously held behavior. At minimum, the saved accepted
contracts still identify these dependencies:

| Area | Owning ADRs | What remains |
| --- | --- | --- |
| Worker ownership and recovery | 512, 513, 514 | Evidence-backed lease, heartbeat, recovery, cleanup takeover, and retry/backoff values where explicitly undecided. |
| Audio processing | 515 | Measured preset values and bitrate meaning/tolerances before held transforms are enabled. |
| Discovery and source identity | 516, 535 | Approved scheduler/traversal/hash budgets and aggregate semantics, then durable implementation and fault validation before automatic or held destructive behavior. |
| Native execution identity | 519, 558 | Full execution-closure verifier and immutable job binding, plus E1 package environment proof/approval and broker containment evidence. |
| Database baseline | 522, 541, 551 | Final init lifecycle/role grants, pristine-schema and baseline verification, and coordinated retirement of migration authority. The assembled candidate alone is not a cutover. |

This is a known-hold list, not a claim to have exhaustively re-audited all
accepted ADRs. Before activating the replacement goal, complete the decision and
requirement ledger against the chosen repository revision. Do not turn a held
included capability into an excluded capability to declare success.

## Proposed Replacement Goal

Complete and merge Revaer's full first-release media transcoding feature as a
production-usable v0 service, implementing the included scope in
`MEDIA_TRANSCODING.md` under operator-approved ADRs and explicit scope amendments.
Deliver the real outside-in operator workflow from configuration and discovery
through inspection, planning, explicit execution, verification, safe replacement,
backup/quarantine, and recovery; retain dry-run-by-default safety without reducing
the release to dry-run only. Finish the single-init database cutover and prove
the supported Linux amd64 and arm64 packages work on clean installations. Use
one linear stack of independently reviewable deliverables, each at most 10,000
added-plus-deleted lines, and merge in dependency order only when authorized and
all applicable gates pass on the exact revisions. Require passing `just ci`,
`just ui-e2e`, all applicable GitHub checks, strict Sonar with real positive
coverage and retained evidence, resolved actionable PR feedback, and exercised
failure/recovery paths before claiming completion. Obtain decision-specific
approval for unresolved architecture; never invent approval, weaken criteria,
silently drop included scope, or claim absolute perfection. Prioritize the
critical path, parallelize independent approved work, keep the requirement and
blocker ledger current, preserve user changes, remove completed task worktrees,
and delete acquired/generated test media after every turn.

### Completion Evidence

1. A revision-pinned traceability ledger maps every included requirement to its
   approved decision, implementation, positive/negative acceptance tests,
   runtime/package evidence, and owning PR. No required row is missing, held,
   unimplemented, or supported solely by a mock when real execution is required.
2. Fresh-install acceptance exercises profile/target/policy/compatibility and
   logical-root configuration, portable YAML, manual discovery, and
   disabled-by-default but functional operator-enabled watchers and schedules.
   API, OpenAPI, UI, SSE, and diagnostics agree with the shipped behavior.
3. Real fixtures prove every claimed copy/remux, rewrite, subtitle, audio, and
   video operation and configured preservation/removal/ordering behavior for
   streams, metadata, HDR, chapters, sidecars, and attachments. Capability-bound
   plans explain selection/rejection; outputs are independently inspected and
   verified before any in-place replacement.
4. Fault tests prove the approved bounds and outcomes for cancellation, timeout,
   process/descendant cleanup, source/root replacement races, insufficient disk,
   concurrent stale owners, crashes/restarts, rollback, backup/quarantine, and
   retention. Untested support claims or cleanup assumptions are not evidence.
5. A clean database initializes from the one authoritative init script. Historical
   migrations are retired only in the coordinated approved cutover; no migration
   is added to work around a frozen baseline. Supported release images include
   the verified redistributable toolchain and digest-bound compliance evidence.
6. Every applicable check actually runs and succeeds on the final PR revision
   and integrated result. Missing, skipped, stale, or untriggered required checks
   are not passes. Sonar scope, security rules, coverage criteria, and branch
   rules remain intact; no new exemption is inferred from this goal.
7. Each PR has a conventional-commit-style title, no prohibited branch/title
   wording, Vanna assigned, Copilot review requested, and all actionable feedback
   addressed with evidence. Keep one linear stack, not branches of stacks;
   scope PRs as outside-in deliverables and observe the 10,000-line ceiling.
   Keep sizes approximately balanced where dependency boundaries permit. Merge
   only through the applicable authorization and protection rules.
8. Operator documentation, support boundaries, risk/rollback instructions, and
   a reproducible release acceptance report match what was actually exercised.
   Cleanup verification covers task-owned worktrees, processes, databases, and
   acquired/generated media while retaining non-media test and review evidence.

### Delivery Discipline

- Resolve decisions needed for the next runnable vertical slice first; batch
  related evidence-backed choices for review instead of producing an indefinite
  sequence of tiny ADRs. Continue independent already-approved work meanwhile.
- Use isolated worktrees and agents for disjoint work. Do not refactor unrelated
  subsystems or pursue undefined perfection. Track a bounded next deliverable,
  its dependency, acceptance evidence, and current blocker at each handoff.
- Use GitHub CLI/API for PR monitoring, not browsers. Use the repository's
  NVM-aware Node wrapper and canonical Justfile gates; do not substitute local
  ad hoc green commands for required checks.
- Revalidate after restacking or changes to the reviewed revision. Diagnose
  repeated failures from evidence; stop retrying an unchanged failing condition
  without a concrete new action.
- This is a proposed statement for the next goal, not an activation command or
  approval of pending ADRs. Record the operator's decisions and remaining holds
  first, then set the agreed objective when the operator asks to pursue it.

## Consequences

Accepting the recommendations would make whole-root discovery representable,
make catalog-rebinding costs explicit, preserve remediation availability only
under proved-safe broker unavailability, and separate genuinely internal work
from architectural oversight. It would not grant environment readiness, enable
held automatic/destructive behavior, shrink the first release, or make CI green.
The new goal sharpens the existing production-usable objective with testable
release acceptance and avoids the earlier absolute-perfection wording while
retaining all quality and approval obligations.

## Task Record

- Motivation:
  - Reconcile the two pending proposals and provide a decision-ready goal after
    the operator requested both, without treating document revision as consent.
- Design notes:
  - Keep accepted predecessor records unchanged; make every requested semantic
    change and implementation-authority distinction explicit and pending.
  - Preserve the original commits and use clean isolated worktrees rather than
    editing the conflicted primary checkout.
- Test coverage summary:
  - No runtime test or code changes. Documentation generation and repository
    validation outcomes are recorded below before this review is handed back.
- Observability updates:
  - No runtime telemetry changes. Existing bounded diagnostics, sensitive-path
    restrictions, and cleanup evidence remain mandatory.
- Status-doc validation:
  - Read the first-release inclusion/exclusion list in `MEDIA_TRANSCODING.md`
    and the owning accepted ADRs. Product guides remain unchanged because no
    capability has been implemented by these proposals. Update the ADR index,
    mdBook summary, and generated document catalogue with the review documents.
- Risk & rollback plan:
  - The risk is mistaking a draft or a green documentation check for architectural
    approval or working media behavior. All new records remain Proposed.
    Rejection removes or supersedes the local document revision; no runtime or
    remote state needs rollback.
- Dependency rationale:
  - No dependency is added or updated. Newly observed dependency-audit failures
    are recorded, not suppressed or repaired incidentally in this ADR-only task.
- Stale-policy check:
  - Reviewed `AGENTS.md`, the ADR template, data/UI/devops scoped instructions,
    and accepted lifecycle/root/identity decisions. The review corrects proposal
    contradictions and exposes remaining holds. It changes no operational
    instruction, workflow, Sonar property, branch rule, or quality threshold.

## Validation Record

- `just instruction-drift`: passed on the documentation worktree.
- `just ci`: failed at the unchanged dependency audit after formatting, policy,
  both Clippy gates, Helm lint, instruction drift, asset verification, and
  unused-dependency analysis passed. The audit reported `h2 0.4.12` under
  `RUSTSEC-2026-0258` and denied the yanked `chacha20 0.10.0` warning. Later CI
  stages, including full tests and coverage, did not run in this attempt.
- `just ui-e2e`: failed in the NVM-aware API-client dependency audit before
  Playwright ran. The audit reported four high-severity dependency entries
  involving `fast-uri`, `js-yaml`, and their dependent packages. No assertion
  about the earlier scheduled-profile failure being fixed is supported by this
  run.
- `just docs-install`: exact pinned tools verified; final combined document
  generation uses `just docs-index` and includes all three review records.
- `just docs-build`: exited successfully but emitted `search index is very
  large` (10,257,112 bytes on the first combined build). This is not a warning-
  free documentation result; no search exclusion or warning suppression was
  added. The generated size can change with final wording.
- `just docs-link-check`: 986 links checked, 986 OK, zero errors on the combined
  review documents and existing documentation tree.
- Read-only peer review found API/YAML lock inversion in the draft. The common
  ordering was corrected and re-reviewed with no remaining direct contradiction
  in those changed sections. This is document review, not runtime concurrency
  proof. The broker subtask's review also exposed a pre-existing root/scoped
  precedence contradiction in `rust.instructions.md`; root policy governs and
  the unrelated instruction file is unchanged here.
- `git diff --check`: passed. Final packaging retains the review documents and
  their local commit without publishing them to GitHub.
- `just clean-test-fixtures`: passed. No real conversion fixture was acquired or
  conversion executed. Policy tests used their own transient fixtures.
- These failures block repository completion. No audit exception, changed
  dependency graph, relaxed check, remote PR mutation, or new-goal activation
  is part of this documentation revision.
