# Evidence-Led Delivery Governance

- Status: Accepted
- Date: 2026-09-11
- Operator approval: "Agreed, make it so" in direct response to the eight
  long-term project recommendations, repeated after an interrupted turn.
- Supersedes: The requirement in root policy section 5 and
  [ADR 461](461-adr-task-record-status-semantics.md) to put every new routine
  task record in the ADR catalogue. Approval/status distinctions and historical
  records remain intact.
- Implementation status: Policy/documentation implemented in `d5662941`;
  integration validation remains blocked. Acceptance of this decision does not
  establish completed verification or delivery of the media feature.

## Context And Decision

The operator approved these recommendations as a delivery approach:

1. Order work around the complete operator journey, using a working slice as an
   early milestone without dropping the remaining specification.
2. Reduce work in progress, use smaller reviewable deliverables within one
   linear stack, and parallelize only independent approved work.
3. Make original-file preservation and real failure/recovery evidence central
   to acceptance, not just successful return values.
4. Keep one requirement-to-evidence ledger and distinguish implementation,
   local verification, package verification, and merge state.
5. Publish an evidence-backed operating envelope for media, hardware,
   filesystems, resource assumptions and supported Linux architectures.
6. Measure operator outcomes; propose numerical reliability/resource targets
   from measurements for operator approval rather than inventing them.
7. Separate new routine execution records from architectural ADRs, retaining
   one explicit approval register and exact supersession links.
8. Rehearse clean install, backup/restore, package replacement and recovery;
   finish the approved single-init cutover before adopting supported subsequent
   schema evolution for persisted user data.

Keeping every iteration in a new ADR was the previous process. Removing durable
records or delegating general architecture authority was not recommended and
is not approved. New execution records use `docs/tasks/`; this is a file-layout
implementation of the approved separation, not a new application subsystem.

## Exact Approval Boundary

This approval changes delivery governance and evidence organization. It does
not approve C1, D4, D5, E1, S2, ADR 585, outstanding numerical choices, new
runtime behavior, new dependencies, new support claims or criteria exceptions.
It does not change GitHub required checks, workflows, Sonar properties/server
settings, quality thresholds, the canonical PR size rule or release scope.

The [approval register](../media-approval-register.md) links exact decisions;
the existing [completion ledger](564-media-completion-ledger.md) stays the single
requirement tracker. The [release verification matrix](../media-release-verification.md)
defines evidence to collect, not evidence already obtained. Existing historical
ADRs remain addressable; no bulk migration or status rewrite is authorized.

## Consequences And Follow-Up

Routine iterations no longer require a new architectural record. They retain
scope, validation, risk, observability, dependencies and stale-policy checks.
Architectural choices still require a Proposed ADR presented before affected
implementation and actual dated operator consent before acceptance.

The next delivery remains init/root/profile integration, manual discovery and
dry-run, followed by complete execution/recovery and package acceptance. The
separate decision holds and current failures remain visible. No gate is waived
for documentation-only work and no historical pass certifies this change.

## Task Record

- Motivation: Turn the approved recommendations into actionable repository
  policy and a single navigable delivery/evidence structure.
- Design notes: Update root policy; introduce a routine-task index/template;
  link the approval register, verification matrix and existing ledger; preserve
  all historical records. Correct the spec's stale assertion that no operator
  decisions remain, without altering included scope.
- Test coverage summary: Documentation index, instruction drift, whitespace and
  all 1,241 links passed. The book builds but retains its large search-index
  WARN. Governance `d5662941` changes no production code or criteria. Its Linux
  CI exposed three existing mode-conversion lint errors, corrected separately
  in [the portability record](../tasks/2026-09-11-root-catalog-mode-portability.md).
  On source `c8d83bf1`, full `just ci` exited zero with all 18 Rust package
  coverage thresholds met, but eight config-watcher shutdown WARN lines remain.
  Full `just ui-e2e` exited one: 46 passed, one failed, 61 not run, plus missing
  phase/readiness route coverage in teardown. Do not claim a clean handoff,
  package acceptance, published Sonar coverage or release completion.
- Observability updates: No runtime telemetry change. The verification matrix
  records measurements needed before numerical targets can be proposed.
- Status-doc validation: Reviewed the specification's open-question section,
  existing completion ledger and ADR 461; synchronized the document navigation.
- Risk & rollback plan: Main risk is misreading process approval as runtime
  consent. Exact holds and approval boundaries remain explicit. Revert this
  bounded documentation change to restore the previous record layout; do not
  erase operator approval evidence or alter runtime/data state.
- Dependency rationale: No new dependency, service, tool or application owner.
- Stale-policy check: Reviewed AGENTS.md, the scoped devops/data instructions
  relevant to retained holds, ADR template, ADRs 461/559 and the
  instruction-drift command. Removed the
  contradiction between routine records and architectural decisions; added the
  missing stale-policy section to the ADR template. Existing gate policies and
  operational source-of-truth files remain unchanged.
- Cleanup: Both completed agent worktrees were removed. The canonical fixture
  cleanup ran; owned validation containers and database volumes were removed.
  Final integration-worktree cleanup follows the retained fast-forward. Primary
  checkout conflicts and separately pending architecture remain untouched.

### Retained Evidence

Local reports and raw coverage are retained outside temporary worktrees at
`artifacts/media-verification/2026-09-11-delivery-governance` in the primary
checkout, including initial failures and successful retries. This ignored
directory is local evidence, not a published CI artifact or Sonar acceptance.
Final documentation-only result updates do not change the tested source.
