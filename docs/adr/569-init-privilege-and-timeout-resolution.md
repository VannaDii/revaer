# Init privilege and timeout resolution

- Status: Accepted
- Date: 2026-09-10
- Operator approval: 2026-09-10: "Approve **D1, D2, conditional D3, S1, and narrowly scoped F1**. Keep **S2 held** pending a defensible shutdown bound."
- Context: ADR 566's real PostgreSQL 16.14 proof found an extension-privilege
  ambiguity in ADR 551 and a confirmed timeout-scope incompatibility. Final
  approval review also found an unapproved compiler-setting parity exception.
- Decision: D1 and D2 accepted; D3 accepted conditional on the independent
  semantic proof specified below. Runtime cutover remains disabled pending all
  required conformance and application evidence.
- Consequences: D1 explicitly chooses the authored-versus-extension privilege
  boundary; D2 and D3 change exact legacy parity and setting-scope constraints. These
  are operator decisions, not G1 internal refinements.
- Follow-up: Implement only the accepted
  delta, regenerate its exact init digest, and repeat all conformance gates.

## Approval Resolution

The operator accepted D1, D2 and conditional D3 exactly as reviewed at commit
`1d62d087`. D1's pinned extension boundary and expiry remain mandatory; D2
changes only reset timeout scope; D3 requires independent cold/warm ingestion
evidence and a renewed decision if any further semantic delta is necessary.
The proposal wording below preserves the reviewed recommendations and prior
hold history; it does not leave these decisions awaiting approval. Approval
does not establish behavioral equivalence, certify a final digest, activate
the single-init cutover, or release any unrelated hold.

## Observed Conflicts

The source for this review is the inert finalization in commit `b74d6b1a`
(worker commit `d451169c`). Its final-init SHA-256 is
`d27d2d99a0957b2502d0461b27c29ed1f2a53e486d1a140e7aff8e5429d7f069`.
The proof completed 64 checks: 62 passed and these two failed. This digest is
not a release certification and has not been embedded or deployed.

1. **Extension execution ambiguity.** The constrained owner installs trusted
   `pgcrypto` and `unaccent`, but 40 extension routines belong to PostgreSQL's
   bootstrap superuser. They retain PUBLIC EXECUTE ACLs in `public`, which the runtime must
   be able to use for application procedures. Omitting explicit runtime grants
   does not remove those ACLs. This catalog result does not prove every routine
   can be invoked directly: two have internal-only callback signatures. The constrained owner cannot
   revoke privileges on those superuser-owned routines. This is PostgreSQL's
   documented [trusted-extension ownership behavior](https://www.postgresql.org/docs/16/sql-createextension.html),
   not evidence that Revaer needs an unrestricted bootstrap account. ADR 551
   explicitly restricts authored routines; its extension-capability and broader
   default-privilege wording do not unambiguously decide inherited extension
   execution. The new proof selected the stricter interpretation. It must not
   be presented as an already-approved zero-extension-execution requirement.
2. **Timeout leakage.** The seed path calls
   `revaer_config.factory_reset_without_media_defaults_v1`, whose existing body
   executes `set_config('lock_timeout', '5s', true)`. It leaves the encompassing
   transaction at `statement_timeout=2min`, `lock_timeout=5s`, and
   `idle_in_transaction_session_timeout=30s`, instead of `2min/2min/30s`.
   Preserving that body unchanged conflicts with the caller-owned timeout rule.

The proof rejects both outcomes. Separately, its reference normalization already
contains D3's unapproved substitution. That is a local prototype mistake, not
consent or a semantic proof; it has not been pushed, embedded, or used by runtime.
No role privilege, extension ACL, timeout value, frozen migration, workflow, or
remote setting was changed to conceal the two failures. Independently passing
pristine-catalog and baseline-reader tests resolve none of these approval gaps.

## D1: Clarify The Extension Privilege Boundary

**Recommendation, not authorization:** retain the pinned, stock `pgcrypto` and
`unaccent` objects in `public`, including their inherited PUBLIC EXECUTE ACLs,
as PostgreSQL extension primitives. Require owner-owned SECURITY DEFINER and
explicit-only runtime grants for every **authored Revaer routine granted to
runtime**. Keep trigger routines and the baseline seal ungranted. Runtime
still receives no application table/sequence access, DDL, extension management,
role administration, or baseline-seal permission. This does not promise that
runtime SQL cannot invoke hashing, cryptography, randomness, or text-search
primitives supplied by PostgreSQL and these two extensions.

The exact clarification and proof replacement requested are:

- Narrow ADR 551's "every runtime-executable routine" acceptance clause to
  authored, directly granted non-trigger routines, with the stock extension
  members independently constrained below. Its authored PUBLIC-revocation and
  explicit application-grant rules remain unchanged. This is the exact broader
  wording D1 supersedes, not an assertion that ADR 551 already made this choice.
- Replace only the new `no effective extension routine grants` assertion with
  an exact pinned-extension inventory/definition/ownership/ACL proof. The
  observed inventory has 40 routine identities, all SECURITY INVOKER; distinguish
  ordinary callable functions from internal callbacks. Derive and compare the
  complete inventory from the pinned clean fixture, not a permissive name glob.
- Preserve every authored-routine grant, search-path, relation-access, owner,
  role-substitution, baseline, and extension-DDL denial assertion. No additional
  PUBLIC grant or SECURITY DEFINER extension member is permitted.
- Approval covers only these stock extension objects under ADR 551's exact
  PostgreSQL image/version. It expires for any image, PostgreSQL minor version,
  extension version, member definition, owner, ACL, or extension-set change;
  renewed explicit review and proof are then mandatory. It authorizes no Sonar,
  advisory, coverage, GitHub-check, or other quality-criterion relaxation.

If the operator instead requires zero inherited extension EXECUTE ACLs or zero
extension-mediated computation, reject D1. A separately approved provisioning
or dependency design is then necessary; neither a private schema nor omission
from an explicit grant list establishes that stronger guarantee.

### Withdrawn Private-Schema Recommendation

The first local draft recommended `revaer_extensions` with no runtime schema
USAGE. Peer review found that this was not sufficient for its promised owner-only
dependency access. Numeric `regdictionary` input avoids name lookup, and
`ts_lexize` dispatches the dictionary callback by OID. This conclusion is derived
from the pinned 16.14 [OID conversion](https://raw.githubusercontent.com/postgres/postgres/REL_16_14/src/backend/utils/adt/regproc.c),
[dictionary dispatch](https://raw.githubusercontent.com/postgres/postgres/REL_16_14/src/backend/tsearch/dict.c),
and [callback initialization](https://raw.githubusercontent.com/postgres/postgres/REL_16_14/src/backend/utils/cache/ts_cache.c)
sources; it is not a live reproduction. Fastpath function calls themselves do
check both namespace USAGE and function EXECUTE in the pinned
[fastpath implementation](https://raw.githubusercontent.com/postgres/postgres/REL_16_14/src/backend/tcop/fastpath.c).
Existing prepared lookups are another limitation of post-hoc schema revocation,
as documented under [schema USAGE](https://www.postgresql.org/docs/16/ddl-priv.html).
No private schema was implemented. It is no longer the recommendation.

Alternatives considered:

- Retain stock extension primitives while strictly protecting authored Revaer
  state and procedures: recommended above, with its explicit capability limit.
- Give bootstrap superuser or ownership of extension member objects: materially
  widens authority and may violate extension safety assumptions; not recommended.
- Add a privileged pre-provisioning or ACL-management step: could enforce literal
  member ACL revocation, but changes pristine admission, installation topology,
  and transaction authority. It needs its own exact operator-approved design;
  no such step is included in this proposal.

## D2: Function-Scoped Reset Lock Bound

**Recommendation, not authorization:** attach `SET lock_timeout = '5s'` to
`revaer_config.factory_reset_without_media_defaults_v1` and remove its redundant
transaction-local `set_config` call in the final init only. This preserves the
existing five-second bound within reset while restoring the caller's exact
prior value on routine exit. PostgreSQL documents this
[function-local configuration scope](https://www.postgresql.org/docs/16/sql-createfunction.html).

The requested supersession is narrow: ADR 551's legacy-definition parity may
include this exact routine configuration/body delta, and its prohibition on
later timeout changes permits this tighter function-scoped lock bound. The
initializer still owns `120s/120s/30s`; no top-level reset, larger timeout,
automatic retry, deadline restart, or post-call repair statement is permitted.
Runtime reset calls also stop leaking their five-second setting into subsequent
statements. That observable scope change is explicitly part of D2.

Removing the five-second bound entirely would change contention behavior;
restoring 120 seconds with a later top-level statement would conceal leakage
and violate caller ownership. Neither alternative is included. The frozen
migration corpus remains unchanged under every option.

## D3: Function-Local Variable Resolution

**Recommendation, not authorization:** permit only the exact
`public.search_result_ingest_v1` substitution from function configuration
`SET "plpgsql.variable_conflict" TO 'use_column'` to an initial
`#variable_conflict use_column` compiler directive, conditional on proving the
complete search-ingestion call paths under the constrained roles. Grant no
superuser or parameter-setting privilege. Change no frozen migration, other
routine body, or ambient/session setting.

This expressly requests an additional exception to ADR 551's legacy-definition
parity constraint. It is not one of its specified SECURITY DEFINER, ownership,
or search-path changes, and G1 cannot supply consent. ADR 566 originally claimed
equivalence; that claim is withdrawn. `final_sql.rb` performs the substitution
and `final_proof.rb` applies it to the comparison reference. That equality check
cannot independently prove either authorization or behavioral equivalence.

PostgreSQL's [PL/pgSQL compilation documentation](https://www.postgresql.org/docs/16/plpgsql-implementation.html)
distinguishes a GUC affecting subsequent compilations from a directive affecting
only its containing function. A helper or trigger first compiled inside the
old function can therefore have a different ambient setting. This is a
source-supported risk, not a reproduced application regression. The current
focused application-path proof does not exercise search ingestion.

Approval must be followed by pinned 16.14 cold-session and warm-cache evidence
for the actual ingestion branches and every reachable helper/trigger, including
ambiguous names and dynamic calls. Compare results, mutations, errors, and
before/during/after setting scope against the frozen reference. If equivalence
cannot be established for application behavior, stop and present the exact
additional semantic delta; do not silently annotate other routines or broaden
privileges. Rejecting D3 leaves the local candidate uncertified and unpublishable.
The exception covers only this routine under the pinned PostgreSQL image and
validated helper/trigger closure; it expires if their definitions, compilation
context, or pinned PostgreSQL identity change. Renewed review is required then.
No further implementation or activation of this substitution is authorized now.

## Acceptance Evidence Required After Approval

- Fresh init under the pinned constrained owner; direct owner/runtime/outsider
  sessions; no role substitution; no added role or server capability.
- For D1, prove the exact stock extension inventory and permissions, no broader
  extension or authored capability, all existing application-state/DDL denials,
  and successful application paths before and after owner NOLOGIN. Retain
  schema/seed parity and complete transaction rollback proof.
- For D2, prove exact before/inside/after values for successful reset, lock
  contention, SQL failure, cancellation, transaction rollback, and nested calls.
  Preserve the fixed whole-operation deadline and final cancellation reason.
- For D3, independently establish the complete cold/warm ingestion call-path
  evidence above. A normalized-reference comparison alone is insufficient;
  restrict any parity exception to the single exact approved substitution.
- Mutation tests must reject changed extension membership/definitions/ACLs,
  authored PUBLIC execution, broadened search paths, timeout leakage, and removal
  or increase of the scoped bound. Existing failing proof assertions remain until
  the operator accepts the exact replacement contract and tests.
- Run full application parity, `just ci`, `just ui-e2e`, positive published
  Sonar coverage and applicable GitHub checks on the resulting stack revisions.
  Freeze/embed no final digest and enable no cutover before all required proof.

## Approved D1/D2 Implementation Checkpoint

The approved final SQL changes only D2's exact function-scoped reset timeout:
one `SET lock_timeout TO '5s'` function option replaces the transaction-local
body call. The regenerated local candidate digest is
`1a9f0f2e9b4ca04ee880baa1babc4f105d2726ee5b4a4fe627f04fcc1a06beb9`.
This is an evidence identity, not a certified or activated baseline. The
167-file frozen migration corpus and runtime bootstrap remain unchanged.

The finalizer now applies D2/D3 transformations only inside the exact named
SQL statement, and rejects missing, repeated or unexpected routine envelopes.
The privilege proof compares the pinned clean extension fixture with the
candidate, including all extension memberships, the 40 routine definitions,
owners, effective ACLs, the two internal callback signatures, and dictionary
and template definitions. Eleven deliberately changed extension states must
be detected, and each mutation must roll back to the exact prior inventory.
All previous authored-privilege and application-state denial checks remain.

`just db-init-final-proof` passed 105 live checks on PostgreSQL 16.14, including
real owner/runtime resets, exact before/inside/after timeout values, nested
reset scope, SQL failure, externally delivered cancellation, independently
held-lock contention, transaction rollback, and application calls after the
bootstrap owner is NOLOGIN. The first harness run exposed its SQLSTATE-only
notice formatter and an invalid self-cancellation assumption across the
SECURITY DEFINER boundary; corrected test observation uses full retained
notices and an exact-backend signal from the disposable test administrator.
No production role was elevated. A no-op dictionary mutation was replaced by
an actual dictionary-definition mutation before the successful rerun.

`just --command bash scripts/tests/database-final-test.sh` passed 18 exact-byte
assertions, including rejection of removed/increased reset bounds and restored
timeout leakage. `just instruction-drift` passed. The 105-check result does not
include or discharge D3's independent ingestion closure proof. Full integrated
CI/UI, published Sonar and GitHub checks are not established by this checkpoint.

## Independent D3 Counterexample

The integrated proof on the same `1a9f0f2e...6beb9` final digest now executes
123 assertions: 122 pass and `ingestion warm-committed required outcome` fails.
All 105 preceding assertions pass again. The original frozen function retains
its function GUC in the independent reference; it is not rewritten to D3's
directive for these behavioral comparisons. Six cold cases match observed
results, errors, eighteen-table state and caller settings. A second ingestion
after commit on the same backend fails in both reference and candidate with
`42P07: relation "tmp_policy_rules" already exists`.

This shared legacy failure neither demonstrates a D3-only regression nor
fulfills the required usable warm path. The proof retains raw observations and
explicitly reports `complete=false` and `passed=false`. Its 44 harness assertions
pass. Independent review found and corrected unconditional rollback of a
successful second call and insufficient control-error validation. Three live
transaction controls prove persistence of successful second writes, visibility
of differing writes and rollback of the expected division-by-zero failure.
Controls require exact SQLSTATE sequences and diagnostics, rejecting warnings.
The final source re-review found no further issues in that correction; it is
not a second independent live rerun.
Complete branch/helper closure and in-call compiler-setting observations
remain outstanding. [ADR 579](579-ingestion-policy-temporary-table-lifetime.md)
presents D4's exact additional temporary-table lifetime delta for approval;
conditional D3 does not authorize that change. No D4 SQL is implemented, no
frozen migration is edited, and the final init remains inert and unpublishable.

## Approved Correction Comparison Checkpoint (2026-09-11)

This continues D3 verification after the [ADR 588 approval](588-first-release-decision-package.md#approval-resolution).
It changes the disposable proof only, not final SQL, migrations, privileges,
runtime bootstrap or the conditional D3 acceptance boundary.

- Motivation: Raw parity rejects the two separately approved D4/D5 fixes. The
  proof must recognize their exact effects without rewriting the reference or
  treating its failed calls as successful application behavior.
- Design notes: Recognize only `warm-committed` and `existing-external-id`.
  Retain `equivalent=false` and record the named approval separately. Pin the
  original SQLSTATE, statement digest, routine, line and native location;
  require exact successful results and all 18 final table images, including
  unchanged rows, generated identity links and transaction provenance. The
  D4/D5 correction matrix remains mandatory before these comparisons.
- Test coverage summary: On parent `b5c860eb` plus the retained proof delta,
  `just db-init-final-proof` passes 371 of 372 checks using pinned PostgreSQL
  16.14. All 21 current ingestion comparisons meet their required outcomes;
  19 retain parity and two explicitly validate the approved correction. The
  remaining failure is the complete conditional D3 scope gate, not a pass.
  The 27-case, 220-check independent correction matrix passes again. Harness
  tests pass 316 assertions, including 82 new checks rejecting extra table
  writes, changed diagnostics/settings/results, lost counterexamples and
  unrelated case names. Exact finalizer guards pass 40 assertions.
  Full `just ci` exits zero, including all 18 package coverage gates, script
  coverage and release build, but retains eight config-watcher shutdown WARNs;
  this is not warning-free release acceptance. Rust LCOV contains 94,524
  covered records of 101,615. All five changed Ruby files have positive generic
  coverage. Full serial `just ui-e2e` still fails profile creation (201 expected,
  400 received): 46 passed, one failed, 61 not run, plus missing job-phase and
  profile-readiness route coverage. Earlier UI attempts stopped at an occupied
  port and concurrent asset generation; logs are retained. No assertion changed.
- Sonar: Whole-file Ruby MAIN analysis through `analyze_code_snippet` reports
  zero issues for all five changed Ruby files in `VannaDii_Revaer`.
  `just --command sonar analyze secrets <changed-files>` passes after retrying
  outside the keychain/cache sandbox. This supplements, not replaces, the
  canonical scanner and positive published coverage, which remain outstanding.
  Documentation links pass 1,333 checks; instruction drift and diff checks pass.
- Observability updates: Retain both raw variants and the explicit approval
  field. `complete=false` and `passed=false` remain until full D3 proof exists.
  No production telemetry changes.
- Risk and rollback plan: An overly broad exception could hide a regression.
  Exact fixture-specific predicates and mutation tests limit this risk; remove
  only this proof delta to restore strict raw equality. Never modify the frozen
  reference or enable cutover to make the proof pass.
- Dependency rationale: No new dependency; use existing Ruby standard library,
  SQL statement parser, pinned PostgreSQL and canonical Just commands.
- Stale-policy check: Reviewed root, data, DevOps and Sonar instructions and
  ADRs 569/579/583/588. DevOps now describes the precise approved comparison
  without weakening full D3 verification. Earlier counterexample records
  are historical; this checkpoint does not claim clean full CI/UI, canonical
  Sonar, package qualification, publication or merge.
- Remaining work: The bounded closure audit identifies mutating logger-first
  calls, populated policy matchers and casts, in-call setting observations,
  and discriminating warm wrapper/scoring/paging paths as the next executable
  batch. Its catalog inventory is not behavioral proof or D3 completion.
- Evidence and cleanup: Exact tested source hashes, patch, all proof JSON,
  gate logs, local coverage and the closure audit are retained under
  `artifacts/media-verification/2026-09-11-d3-adjudication/`. UI teardown removed
  its last task database and media; the canonical fixture cleanup passes.
  No production media, frozen migration, user checkout or remote PR changed.

## In-Call Setting Proof Checkpoint (2026-09-11)

- Scope: Proof-only continuation from `62c5d6a7`; no final SQL, frozen
  migration, privilege, runtime or acceptance-criteria change.
- Motivation and design: Exercise the real conflict logger before ingestion
  on a cold tested connection, including NULL fallback and 256-character
  truncation, then reuse that connection across commits. Also exercise cold
  canonical insertion, nested conflict logging and a persisted policy flag
  through the real decision cast. Disposable triggers observe session/current
  role, backend, transaction and ambient setting at three write boundaries.
- Verification: Four cases, two database variants and paired plain/observed
  runs pass 140 checks. All 18 application tables compare per operation;
  instrumentation must preserve behavior. Validate D4's exact temporary-table
  lifetime separately before normalizing that metadata. Generated identities
  and named transaction columns normalize only after validation; arbitrary
  UUID-shaped text and other timestamps remain visible. Harness tests pass
  515 assertions. Full `just db-init-final-proof` passes 511/512 checks; its
  sole failure remains the incomplete-D3 gate. These observations cover
  committed successful writes, not failed-call or complete trigger closure.
- Counterevidence retained: The first draft miscounted the two distinct hash
  conflict records and used `decision_type` instead of the persisted `decision`
  column. The harness was corrected against the unchanged routine/schema;
  neither failed assertion nor initial evidence was hidden.
- Observability: Raw queries, diagnostics, full images, observer events and
  real Ruby coverage are retained. No production telemetry changes.
- Risk and rollback: Observers could affect compilation or writes; paired
  uninstrumented runs and exact state comparisons guard that risk. Remove
  only this proof delta to roll back. Never use it to enable cutover early.
- Dependencies: Existing Ruby standard library and pinned PostgreSQL only.
- Stale-policy check: Reviewed root, data, DevOps and Sonar instructions and
  ADRs 569/588. DevOps records the bounded evidence contract. Historical
  checkpoints remain historical, not fresh release or approval claims.
- Remaining work: Populated policy matchers, discriminating warm wrapper and
  paging branches, remaining errors/mutations, and native/trigger closure.
- Full gates on the proof-only tree: `just ci` exits zero with all 18 package
  coverage gates and the release build, but retains eight config-watcher
  shutdown WARNs. Rust LCOV records 94,557 covered lines of 101,615 records.
  The merged real script coverage includes 173/174 lines in the new compilation
  module and 107/107 in its tests. `just ui-e2e` again reports 46 passed, one
  failed and 61 not run: profile creation returns 400 instead of 201, and
  teardown rejects missing job-phase/readiness GET coverage. The initial CI
  instruction-drift failure and UI database-ownership setup failure are retained;
  the reruns use the matching instruction update and documented caller-managed
  disposable database option, not reduced tests or relaxed assertions.
- Sonar and review: All five changed Ruby files return zero issues from
  whole-file MAIN analysis after a retained DNS-failure retry. Secrets analysis
  passes. `just sonar-scan` stops because `SONAR_TOKEN` is unavailable; no
  canonical scan or published-coverage pass is claimed. Independent review
  found no actionable issue and rejected 52 additional evidence mutations;
  it replayed retained results, not a second live PostgreSQL run. Finalizer
  guards pass 40 assertions and documentation links pass 1,333 checks.
- Evidence: Source hashes, exact dirty delta, initial failures, final raw proof,
  gate logs, local coverage and review are retained in
  `artifacts/media-verification/2026-09-11-d3-compilation/`. No runtime SQL or
  frozen migration changed. C1 was implemented independently at `81947a65`;
  this proof-only CI/UI run does not qualify the combined integration, packages,
  publication, merge or complete service.

## Wrapper And Warm-Cache Proof Checkpoint (2026-09-11)

- Scope: Continued D3 proof from integration `18357155`, with no final SQL,
  frozen migration, runtime, authority, dependency or criterion change.
- Motivation and design: Add the actual Rust-called `search_result_ingest`
  wrapper to the exact routine inventory. Compare its first visible-source
  tail, competing stored scores and seeder thresholds, a full ten-item page
  and its successor, canonical reuse without page mutation, the third size
  sample, and retention of the latest 25 samples after the 26th observation.
  Fixture rows come from successful ingestion on separate recorded backends;
  explicit base scores are pinned read inputs, not fabricated procedure results.
- Warm-cache boundary: Each of eight cases runs cold, after pure-helper first
  use, and after a real same-backend ingestion rollback followed by a committed
  retry. Rollback is observed through full table images; no DROP, DISCARD,
  reconnect, role substitution or compiler-setting change repairs the tested
  session. This is warm-after-rollback evidence, not successful frozen
  committed reuse. The separate exact D4/D5 correction matrix remains mandatory.
- Validation: All 48 database-variant runs and 24 paired comparisons pass
  456 checks. The actual canonical `just db-init-final-proof` passes 967/968;
  only complete conditional D3 remains unproven. All 18 write tables compare
  per operation. Six read-input relations are retained and immutable across
  tested calls; only explicitly named input timestamps equal to the recorded
  seed transaction may normalize. Other input values and unobserved timestamps
  remain visible. This does not claim a complete read/native dependency closure.
- Harness and analysis: 619 assertions pass, including deliberate mutations
  of result identities/flags, every write-table image, stored scores, title
  selection, page seal/position, retained samples and input clock provenance.
  Finalizer mutation guards pass 40 assertions; instruction drift and whitespace
  checks pass. All four changed Ruby files return zero issues from full-file
  MAIN Sonar analysis; both review-corrected files were rescanned with zero
  issues. `just script-coverage` passes. Merging its executed records with the
  instrumented canonical proof records gives wrapper coverage of 139/139 lines,
  wrapper-test coverage of 127/127, proof-owner coverage of 211/217 and
  proof-harness coverage of 195/196. This is local measured coverage, not
  canonical Sonar or positive published coverage proof.
- Independent review found that the initial highest-score fixture also won
  the lowest-ID tie-break, so wrong local-variable ordering could remain
  invisible. Reversed the real fixture insertion order and retained a negative
  control: the high-score source has the larger ID; wrong local ordering
  selects the already-100-seeder low-score source and incorrectly leaves the
  previous best unchanged. Initial reports and source are preserved under
  `artifacts/media-verification/2026-09-11-d3-wrapper-initial/`; their numerical
  pass counts alone do not establish this corrected binding proof. The full
  corrected canonical run again passes 967/968 checks, with only the incomplete
  D3 sentinel failing. Independent re-review replayed all 48 runs and 400 raw
  frames, verified all eight decisive calls reject the faulty ordering, and
  found no remaining issue in that scope.
- Scanner boundary: The exact-file Sonar secret scan passed. The authoritative
  `just sonar-scan` exited before analysis because `SONAR_TOKEN` is unavailable;
  no canonical quality-gate or published-coverage pass is claimed.
- Observability: Retain raw queries/results/diagnostics, all transaction and
  input images, source identity and check reports. No runtime telemetry changes.
- Risk and rollback: Test setup or normalization could hide a real difference;
  separate connections, unchanged inputs, exact result predicates and mutation
  tests constrain that risk. Remove only this proof delta to roll back; retain
  the conditional D3 gate and never cut over from this subset.
- Stale-policy check: Reviewed root, scoped data/DevOps/Sonar instructions,
  ADRs 569/588 and the existing proof owners. The DevOps update makes the
  warm-rollback and named-clock limitations explicit; no approval is inferred.
- Remaining: Complete populated policy/error/identity branches and native,
  trigger and read-dependency closure. Full integrated CI/UI, canonical Sonar,
  both native packages and merge acceptance remain required.

## Policy, Identity And Native Proof Checkpoint (2026-09-11)

- Scope: Continue the approved D3 proof without changing frozen migrations,
  final SQL, runtime authority or acceptance criteria. Policy commit `a3657b3b`
  was replayed byte-for-byte as `ff7a681e`; native inventory `dc004eb9` was
  replayed byte-for-byte as `6b335753`. Parent wiring invokes both through the
  existing canonical proof, alongside the new identity matrix.
- Policy evidence: 49 populated cases and 196 variant runs pass 1,472 checks.
  Retain all 18 write tables and 19 read inputs, exact decisions and actual
  `action::decision_type` execution. Exercise require families, precedence,
  matchers, actions, severities and prefer branches. Cold/helper-first and
  warm-rollback evidence remains distinct from approved committed D4 reuse.
  The frozen release-group regex requires a literal-backslash suffix in this
  fixture: ordinary suffix recognition and policy-creation API conformance
  are not proved by these cases. No regex repair is included.
- Identity evidence: Add new/reuse/GUID-promotion cases for six identity
  strategies, with distinct lower-ID decoys and competing v1/v2 identities.
  The full wrapper matrix now has 26 cases, 156 runs and 1,482 passing checks.
  Independent replay validates 724 frames, including 324 identity frames,
  with 3,024 additional assertions and 4,768 targeted mutation rejections.
  Canonical/source/observation identities, refresh data, separate hash vectors,
  unchanged unrelated rows and wrapper best-source selection are explicit.
- Review corrections: Initial validators omitted some observation/refresh
  assertions and skipped the common wrapper tail. Discriminating fixtures
  replaced single-row identity cases. The first corrected live run then caught
  a wrong test expectation: v2 reuse preserves canonical v1 absence but fills
  source v1 from the supplied hash. The SQL was not changed to satisfy it.
  A subsequent coherent wrong-selection mutation exposed result-anchored ID
  selection. The final predicate independently selects the prior fixture
  observation and binds all three expected IDs to it. Twelve modeled mutations
  and 16 raw mutated frames pass the old predicate and fail the new one.
  Independent re-review reports no remaining finding in that bounded scope.
- Native evidence: The worker's pinned PostgreSQL 16.14 run passes 823 checks
  with exact container/volume cleanup. It compares recursive catalog edges,
  definitions/settings, read-relation dispatch, FK equality and trigger
  bindings, actual policy cast, unaccent dictionary/template, and native file
  identities. Real known answers run cold and across commits. Installed
  callbacks and eligible row changes are explicitly not callback-entry traces;
  `pg_depend` is not complete PL/pgSQL late-binding analysis. This run predates
  the parent identity extension and is not combined-proof acceptance.
- Harness: The integrated proof harness passes 1,868 assertions; the new
  dependency harness passes 104. The last pre-native canonical run passes
  3,465/3,466 checks, failing only the explicit incomplete-D3 sentinel. Its SQL
  generation is unchanged by the final identity-predicate fix, which was
  independently replayed against the retained raw evidence. The newly wired
  canonical run on `6b335753` plus this staged proof delta passes 3,469/3,470:
  its only failure is incomplete D3. All four dependency checks pass, tied to
  166 current-process observation files. Finalizer guards pass 40 assertions.
- Native review status: The 3,469/3,470 run predates three required validator
  corrections. Coherent root/callback omissions could pass both catalogs;
  check labels did not bind later-consumed frame bytes; equal invalid backend
  IDs could satisfy warm provenance. The retained data itself replayed
  consistently. Correction `7f7b91c9`, replayed as `4981eef4`, plus the parent
  validation-time byte-hash producers closes those three findings. A fresh
  canonical run again passes 3,469/3,470, failing only incomplete D3. Independent
  replay passes 556 assertions, verifies all 166 file hashes and rejects the
  exact unchanged-707-root callback omission, empty/duplicate frames and 14
  invalid-backend mutations. Another 24 assertions exercise the actual producer
  methods with mocked transport and tampered writes; no observation registry
  was reconstructed from disk. The corrected native unit suite passes 197.
- Analysis and gates: Full-file MAIN Ruby Sonar returns zero issues for the
  final identity module/test, proof owner/harness and native module/test.
  The integrated CI attempt exits zero, including all 18 package coverage gates
  and the release build, but retains eight watcher shutdown warnings and the
  fixture suite's explicit ignored test. Rust LCOV contains 94,718 covered
  records of 101,711. Source changed during that run; it is not a clean final
  revision certification. UI stops at missing C1 startup metadata before any
  browser cases. ADR 586 already permits an explicit test-only injected loader;
  wiring that path is approved implementation, not a new architecture request.
  Canonical Sonar cannot start without `SONAR_TOKEN`; no published coverage,
  full UI, Linux package, remote-check or merge pass is claimed.
  Documentation generation succeeds; `just docs-build` exits zero but emits
  a large-search-index warning (14,668,403 bytes). This is not warning-free
  documentation qualification, and no search or analysis scope was reduced.
- Latest integrated gates: On `4981eef4` plus the retained parent delta, with
  executable source unchanged throughout the run, `just ci` exits zero through
  all 18 package coverage gates, script coverage and the release build. Eight
  watcher shutdown warnings remain, so this is not warning-free qualification.
  Its owned database and anonymous volumes were removed. Full UI now reaches
  57 passed, one failed and 72 not run through ADR 586's explicit test bootstrap;
  legacy schedule enablement and missing UI coverage remain blocking. Full-file
  MAIN Sonar reports zero issues for both corrected native files and both
  producer files; canonical `just sonar-scan` still fails before analysis for
  missing `SONAR_TOKEN`. The later child-coverage fix is tracked in ADR 586 and
  is not retroactively certified by this CI run.
- Observability and evidence: Preserve raw queries, diagnostics, images,
  source boundaries, local execution coverage and review under
  `artifacts/media-verification/2026-09-11-d3-identity-initial/`,
  `2026-09-11-d3-identity-review-intermediate/`,
  `2026-09-11-d3-integration-pre-native/` and `2026-09-11-d3-native/`.
  The native worker did not retain two superseded development failures; their
  final successful replacements do not reconstruct those missing logs.
  No production telemetry or original media changed.
- Risk/rollback and dependencies: Test oracles and normalization can hide real
  differences, so retain deliberate negative controls, separate role/session
  evidence and independent review. Revert only these proof changes to roll
  back; retain the D3 gate and inert init. Use existing Ruby standard libraries,
  PostgreSQL tools and pinned container only; no new dependency is introduced.
- Stale-policy check: Reviewed root, data, DevOps, UI and Sonar instructions,
  ADRs 569/586/588 and the closure audit. DevOps now requires fixture-bound
  expected identities and the exact limits of catalog/native evidence. No
  approval, condition, scanner scope or required check is weakened.
- Remaining: Validation/error, identity-fill/conflict, attribute/signal and
  reachability families plus complete helper/native dispatch qualifications.
  Full stable-source CI/UI, canonical Sonar, both packages and merge remain
  required. Every report retains `d3_complete=false`; no cutover is authorized.

## Task Record

- Motivation: Present the two live single-init conflicts and the subsequently
  identified unapproved parity exception without inventing architectural
  consent or treating a normalized-reference comparison as behavioral proof.
- Design notes: The original proposal identifies predecessor constraints,
  alternatives and retained boundaries. The approved D1/D2 implementation is
  recorded in the checkpoint above. D3 remains conditional on independent
  semantic proof; its local candidate must not be published or activated yet.
- Test coverage summary: ADR 566's original 62/64 result is historical. The
  approved D1/D2 checkpoint and subsequent 122/123 integrated result are
  recorded above; the new D3 warm failure is retained, not accepted as a pass.
- Observability updates: Retain the two named failures and bounded evidence;
  no runtime telemetry, error contract, or credential handling changes.
- Status-doc validation: Reviewed the completion goal and ADRs 522, 541, 551,
  and 559. Single-init remains inert; E1 and other retained holds remain held.
- Risk & rollback plan: D1 explicitly permits stock extension computation and
  requires exact inventory proof; D2 changes nested reset timing. Both are
  implemented in the inert local finalization only. D3 remains held pending
  proof, not a second approval of the same conditional substitution. Reverting
  this task's implementation restores the previous local candidate without
  touching deployed state or GitHub; never repair a sealed baseline in place.
- Dependency rationale: No new dependency proposed. Keep the existing pinned
  PostgreSQL tools, extensions, SQLx transport, and canonical recipe surface.
- Stale-policy check: Reviewed root, data, Rust, devops, and Sonar instructions.
  Tightened data instructions to prohibit publication or activation of the
  uncertified D3 candidate and to distinguish reference normalization from proof.
  No accepted ADR, required check, quality threshold, or frozen migration is
  changed by this proposal. Indexes and generated docs are updated.

## Validation Guard Checkpoint (2026-09-11)

- Motivation: Add the bounded validation/error family from closure-audit case 6
  without changing the frozen ingestion routine or discharging D3.
- Design notes: Worker `13ee74af`, replayed as `021656b4`, adds 108 cases at
  32 independently fixed RAISE coordinates. Cases cover request/indexer
  eligibility, title/hash/identity guards, companion arrays, duplicate keys,
  value channels/types and numeric/text domain failures. Four legitimate
  NULL/empty controls remain accepted. Cold and helper-first sessions repeat
  errors after savepoint rollback and commit on the same backend. Successful
  controls retain first-call rollback, second-call commit and the exact D4
  frozen third-call error versus final success.
- Evidence integrity: Retain exact diagnostics, roles/GUC/backend/clock frames,
  unchanged read inputs and all 18 table images. Register original serialized
  bytes only after validation; missing hashes, changed bytes and framing or
  state mutations remain errors. Sequence rollback and complete helper/native
  closure are not claimed.
- Test coverage summary: Worker units pass 1,426 assertions. Its fresh live
  wrapper passes 872 checks, including 649 validation-matrix checks across
  216 paired cases. All nine recorded source hashes match the committed files;
  exact container and named-volume cleanup succeeds. Earlier failed attempts
  remain in the archived worker evidence. Parent units pass 1,430 assertions
  after four added report-classification checks; the existing harness's 1,868
  and native inventory's 197 assertions also pass.
- Parent correction: The worker labelled successful D4 controls equivalent
  after comparing only their first two calls. Raw third-call differences were
  retained, but that label was misleading. The parent now records
  `equivalent=false`, explicit acceptance and the named approved delta. The
  archived worker result predates this reporting correction. Canonical proof
  and policy-harness wiring are present; fresh integrated evidence and
  independent review are still pending at this checkpoint.
- Observability updates: No production telemetry changes. Preserve source,
  initial/final evidence and review under
  `artifacts/media-verification/2026-09-11-d3-validation/`.
- Risk and rollback: A bounded matrix cannot certify all ingestion behavior.
  Keep `d3_complete=false`, the canonical incomplete-proof failure and frozen
  migration authority. Reverting this proof increment cannot permit cutover.
- Dependency rationale: Existing Ruby, PostgreSQL transport and proof helpers
  only; no runtime dependency, schema, permission or package change.
- Stale-policy check: Reviewed root, DevOps, Sonar and the ADR 569/588 approved
  boundary. DevOps now requires these exact guard/control distinctions and
  canonical wiring. No acceptance criterion or prior hold is relaxed.

### Integrated Validation Result

The fresh canonical proof on `021656b4` plus the parent wiring/report correction
passes 4,118 of 4,119 checks. Its only failure is the required incomplete-D3
sentinel. The new matrix completes all 649 checks: 216 accepted pairs, with
208 equivalent guard cases and eight explicitly non-equivalent approved D4
controls. Its nine declared source hashes still match. This is not a successful
final-init qualification or permission to replace the frozen migrations.

Independent review retains 4,637 assertions, 692 rejected coherent mutations,
eight mocked producer controls and byte-hash readback of 3,030 raw files. It
also checks the parent's report correction independently; no residual actionable
finding remains in this bounded review. The reviewer did not recreate a worker
registry from old files or certify the new canonical run. Reproducible review
code, results, raw inputs and exact limits are retained under
`artifacts/media-verification/2026-09-11-d3-validation-review/`.
The completed worker worktree and reviewer have been removed/closed. Full-file
MAIN Ruby analysis reports zero issues for the new module and its tests;
the two canonical wiring owners also report zero MAIN Ruby issues.

Final `just ci` on the same stable executable source exits zero on 2026-09-12,
including all 18 unchanged package coverage gates, script coverage and the
release build. Documentation alone changed during execution. Eight
config-watcher shutdown WARNs remain; this is not a clean handoff. The exact
owned database `revaer-approved-ci-13d9879c395b` and its anonymous volumes were
removed. Raw coverage and the complete log are retained with the proof evidence.
The fixed 52-file secrets scan ran successfully; canonical `just sonar-scan`
stopped before analysis because `SONAR_TOKEN` was unavailable. Full UI, positive
published Sonar coverage, both native packages and D3 qualification remain open.

## Settings-Path Proof (2026-09-12)

The next bounded increment observes successful rollback/retry, late validation
failure and nested policy-regex failure in cold and helper-first sessions.
Test-only NOTICE records survive rollback; paired uninstrumented runs check
that observers preserve application outcomes. Exact frozen insertion/stack
coordinates, original diagnostics, caller settings after transaction completion,
all 18 table images and the 19 read inputs remain required. The existing
protocol is unchanged unless its explicit finish-setting observation is enabled.

Focused units pass 281 assertions. The fresh isolated run passes 266 checks,
including 43 settings-path checks, and removes its exact container and named
volume. Its source is `af67cfce` plus the five proof/test changes, before later
canonical wiring. The initial missing-notice-context attempt remains retained;
the corrected client requests complete context rather than discarding it.
Independent review and canonical integrated validation remain in progress.

No production telemetry, dependency, schema or frozen-authority change is made.
Rollback removes only this proof increment; the incomplete-D3 failure remains.
Root, data, DevOps and Sonar instructions were reviewed; DevOps now specifies
the exact trace boundary. Neither these tests nor approval imply full D3,
clean CI/UI, package, published Sonar or merge qualification.

Independent review found three validator gaps: NOTICE/error ordering was lost,
matching empty input snapshots passed, and the regex error was not restricted
to the selected title callsite. The parent added combined-stream validation,
independent seed/clock checks and the exact title/helper coordinates, with
regressions. The initial 281-assertion/live result is not evidence that those
gaps were closed; corrected units pass 309 assertions. The superseded canonical
run was interrupted with
exit 130, its owned container was verified absent, and its log was retained.

The corrected disposable run on `52c7ef13` plus the owned settings/wiring delta
passes all 266 checks and removes its exact container and named volume. All 12
declared source hashes match the parent. Independent recheck passes 194 review
and 309 unit assertions, rejects the ten unchanged original witnesses and 16
fresh coherent mutations, and finds no residual issue in the three-finding
scope. Its isolated checkout differs only by six canonical wiring lines in
one declared file; this is retained replay, not a second live integrated run.
All 230 original review artifacts remain unchanged. Exact source selections,
raw output, failed attempts, commands and manifests are archived under
`artifacts/media-verification/2026-09-12-d3-setting-paths/`.

The exact ten-file secrets scan passes. Full-file Ruby MAIN analysis of the
two corrected files and both canonical wiring owners reports zero issues for
`VannaDii_Revaer`. These auxiliary results do not establish canonical scanner,
quality-gate or positive published-coverage acceptance. The combined canonical
result below exercises the integrated wiring on stable executable source.

## Attribute And Signal Proof (2026-09-12)

Worker `90a3c37b`, replayed as `52c7ef13`, adds 12 cases and 48 cold/helper-first
variant runs. It checks every valid non-D5 typed attribute key, normalized
language/subtitles, year/season/episode, release-group acceptance/rejection,
trust ranks 19/20/30/40, real first signals and repeated ingestion. Focused units
pass 206 assertions; the worker's pinned arm64 live driver passes 297 checks.
All 19 declared source hashes match its commit, and all three attempts' exact
containers and named volumes were verified absent. Raw attempts remain retained.

The frozen ordinary-suffix regex defect and duplicate signals caused by nullable
uniqueness remain explicit limitations, not accepted feature completion. No UUID
attribute key is invented; D5 ID cases retain their separate required proof.
Independent review replays all 48 variants and 192 frames, passes 245 review
and 206 unit assertions, rejects 26 mutations, and passes three producer
controls. No actionable finding remains within that scope. Repeated identical
attributes do not prove changed-value conflict updates or metadata conflicts.
Both full Ruby files report zero MAIN-scope Sonar issues. Exact raw attempts,
source, replay, commands and manifests are retained under
`artifacts/media-verification/2026-09-12-d3-attributes/`.

Canonical wiring is present and exercised by the combined result below. The two
completed review worktrees were removed after archival, and the reviewer was
closed. No new runtime, schema, telemetry or dependency change is made.
Rollback removes only the proof increment. Root, data, DevOps and Sonar policy
were reviewed; the DevOps proof contract was tightened without relaxing a gate.

### Combined Proof Result

On `52c7ef13` plus the owned settings and canonical-wiring delta,
`just db-init-final-proof` passes 4,234 of 4,235 checks and exits one. Its sole
failure is the explicit incomplete-D3 gate. All 43 settings-path and 73
attribute checks complete successfully; their 12 and 19 declared source hashes
still match the parent. No executable source changed during the run.

The exact PID-bound disposable container was removed and its absence verified;
the proof's cleanup completed without a recorded failure. Complete raw proof
evidence and the log are retained in the settings-path archive. This closes
neither unobserved identity-fill/conflict and reachability cases nor full
helper/native qualification. The frozen migrations remain authoritative.
Full clean CI/UI, canonical Sonar with positive published coverage, both native
packages and exact-revision PR acceptance remain separate required gates.

Full `just ci` on the same stable executable source exits zero with all 18
unchanged package gates, script coverage and the release build, but retains
eight config-watcher shutdown WARNs. Full `just ui-e2e` again reports 57 passed,
one failed and 72 not run: schedule enablement receives 400
`media_profile_filesystem_identity_required` instead of the required 200, and
teardown rejects missing UI coverage. Both exact gate databases and volumes
were removed. Canonical `just sonar-scan` stops before analysis for unavailable
`SONAR_TOKEN`; positive local coverage is not published Sonar acceptance.
The [coverage checkpoint](586-compliance-manifest-failure-boundary.md#settings-and-attribute-checkpoint-2026-09-12)
records the current inputs and limits. No gate, warning, approval or required
operator behavior was relaxed, and no PR was pushed or merged.

## Missing-Hash Proof (2026-09-12)

This continued D3 task adds six missing-source-hash cases: v1, v2 and explicit
magnet identities, each with an uncontested or competing GUID-less peer. The
target begins without hashes. Independent full-column models specify both
real fixture states and all 18 write tables after ingestion, including exact
identities, result flags, conflict/audit/health effects and sequence gaps after
whole-transaction rollback. A conflicting peer blocks the corresponding durable
hash fill while the incoming observation retains that identity. The derived
magnet can still fill in the v1/v2 cases; this is observed frozen behavior,
not permission to reinterpret source and observation state as equivalent.

The source is `a19e5856` plus the five-file proof/test delta. The fresh canonical
wrapper matrix passes all 1,945 checks, including 18 new paired cases in cold,
helper-first and same-backend rollback/retry modes. Those pairs contain 36
variant runs and 48 tested calls, separate from their 72 fixture calls. All
31 declared proof-source hashes remain unchanged. The earlier 12 cold probes
remain diagnostic history; replaying them with the new assertions is not
relabeled as a fresh database run or full D3 qualification.

The proof test owner passes 2,495 assertions, with the existing dependency,
validation, settings and attribute suites also passing. Full-file Ruby MAIN
analysis of all five changed files reports zero issues for `VannaDii_Revaer`;
the exact five-file CLI secrets analysis passes. These are auxiliary Sonar
results, not the authoritative repository scan or positive published coverage.
Independent review subsequently found two validator gaps: coherently empty or
changed read inputs could be accepted, and fixture results could contain extra
fields. The initial pass above does not close those findings. The correction
independently specifies all six seeded read tables, including column defaults
and empty base scores, requires exactly five fixture result fields, and adds
coherent mutations for every input column and result-schema drift. Corrected
focused suites pass 5,621 assertions, including 3,479 in the proof test owner.
Both corrected full-file Ruby MAIN analyses again report zero issues.

The full `just db-init-final-proof` invocation then exercises 4,698 checks:
4,697 pass and only `ingestion complete conditional D3 proof` fails. The report
retains `completed=false` and `passed=false`; this is not conformance acceptance.
The frozen candidate and final-init digests are unchanged. The owned
`revaer-final-proof-92212-ec72842e01928e87` container was removed with its
anonymous volume, and the exact-name absence check returned no container.
The complete invocation is retained in
`/private/tmp/revaer-approved-canonical-hash-fill.log`; its proof reports and raw
SQL/stdout/stderr remain under `target/database-rebaseline/` pending archival.

That original canonical evidence was archived before the corrected rerun, under
`artifacts/media-verification/2026-09-12-d3-hash-fill/canonical-proof-evidence.tar.gz`.
The independent review retains all 2,160 mutations, including the original 72
accepted invalid controls, and all seven simulated-transport producer controls.
Its proposed correction rejects every mutant; only the valid producer control
registers evidence. In-memory review is not fresh database execution: parent
revalidation of the applied correction and integrated gates remains in progress.
The first CI invocation was interrupted after the findings, exited 130, and
removed `revaer-approved-ci-279889b5ad8f` with its volume; exact-name absence was
verified. Its complete original log remains separate from the corrected gates.

The fresh corrected canonical invocation passes 4,733 of 4,734 checks and again
stops only at the explicit incomplete-D3 guard. Its wrapper matrix passes all
1,981 checks with unchanged declared source bytes. Independent recheck passes
480 checks across all 36 corrected variants and 18 pairs, rejects the original
malformed producer witnesses without registration, and retains the valid
registration control. All 348 earlier review artifacts remain unchanged; no
actionable finding remains in the bounded two-finding review scope. The corrected
`revaer-final-proof-41476-b2544c088a52a19a` container and its anonymous volume were
removed, and its exact-name absence check passed. The corrected full log is
`/private/tmp/revaer-approved-canonical-hash-fill-corrected.log`.

No frozen SQL, runtime behavior, telemetry or dependency changes. Rollback
removes only this proof increment. The root, Rust, data, DevOps and Sonar
instructions were reviewed; the DevOps proof contract was tightened. Stale Rust
guidance permitting credential defaults was replaced with a reference to the
already enforced explicit disposable-database input contract. Remaining metadata
conflict/interleaving and helper/native conditions keep D3 incomplete. Accepted
architecture still does not authorize premature init cutover or weakened gates.

The reviewed proof and instruction correction are committed as `9c861f2b`.
The [hash-fill gate checkpoint](586-compliance-manifest-failure-boundary.md#hash-fill-gate-checkpoint-2026-09-12)
records the fresh full CI/UI results on that clean source: CI exits zero with
eight watcher WARNs; UI fails at schedule enablement and rejects missing UI
coverage. Canonical Sonar cannot run without its token. The complete corrected
proof, independent recheck and gate evidence are archived under
`artifacts/media-verification/2026-09-12-d3-hash-fill/`; the earlier pending-run
statements above remain dated history, not the current result. No remote or
release acceptance is established, and remaining D3 work continues separately.

## Changed Metadata Proof (2026-09-12)

This continued D3 task covers three finite serial cases: replacing all 16
non-D5 typed observation values, adding eight previously absent attributes,
and appending conflicts beside existing conflicts with a stale observation
and a 512-character tracker. Independent models specify all 18 write tables,
19 read tables and defaults, and six affected sequence counters. Each case
runs cold and helper-first against reference and final schemas, with a real
fixture backend, whole rollback, same-backend retry and committed reuse.

The author commit `a6fb4bcc` passed 12,851 assertions and 12 live variants
with 243 checks, including 19 metadata checks. Independent review then found
two false acceptances: duplicate JSON fields discarded before validation and
coherently impossible clocks accepted through evidence registration. These
findings invalidate any claim that the original validator closed those cases.
The two-file correction `0caaf40c` rejects all 36 original actionable witnesses;
12,927 assertions and 37 producer controls pass using retained transports.
That replay is not fresh PostgreSQL execution or independent approval of the
correction. Parent review identified compatibility with older JSON runtimes;
follow-up `9d055ac7` combines the standard object hook and native rejection,
with a fail-closed behavioral check. It passes 12,931 focused assertions and
rejects all 36 original witnesses on retained transports. Decoder-only controls
pass on Ruby 2.6.10 / JSON 2.1.0 and Ruby 4.0.6 / JSON 2.18.0; they do not certify
the full suite on older Ruby. Parent inspected both patches. The integrated
metadata and wrapper source lists include the new guard; fresh gates remain
pending, and the earlier broad producer matrix was not repeated unnecessarily.

The metadata owner retains unique raw SQL/stdout/stderr directories and only
registers successfully validated bytes. The original findings and later
authored corrections remain separate evidence. The author's first failed
live run lost its shared correction-output bytes to a retry; surviving shared
bytes prove only that later run. This provenance gap cannot be reconstructed
or relabeled. Parent source fingerprints now also include the loaded ASSET-1
guard; historical counts and hashes remain historical.

Frozen ingestion only appends conflict, audit and health rows; conflict
resolution/reopen updates are not in this call path. A valid serial non-NULL
durable attribute cannot enter the guarded conflict-update branch. Neither
observation proves concurrent behavior or complete helper/native closure.
The exact frozen third-call D4 failure remains visible, as do the literal
backslash suffix and NULL-distinct duplicate signals. D3 stays incomplete.

No SQL, init authority, runtime behavior or telemetry changed. Rollback removes
only this proof increment. Dependencies remain existing tools and Ruby standard
libraries. Root, data, Rust, DevOps and Sonar instructions and ADR 588 were
reviewed; no approval, criterion or threshold was weakened. Full clean CI/UI,
canonical Sonar with positive published coverage, native packages and current
PR acceptance remain separate requirements.

Fresh integrated verification on executable `5027eba5` and documentation-only
successor `7ee7df07`: 12,932 focused assertions pass; canonical proof passes
4,752/4,753 checks and fails only the explicit incomplete-D3 guard. All 19
metadata checks pass across 12 live variants with 32 source hashes; the wrapper
retains 33 source hashes. Full CI exits zero with eight watcher WARNs. Full
UI/API reports 57 passed, one failed and 72 not run at legacy schedule
enablement; teardown rejects missing UI coverage. Local first-party Rust LCOV
records 122,231 covered of 131,114 lines in 310 files, not published Sonar
acceptance. The canonical token remains absent, and the rejected eight-file
upload was not retried. No native package, PR or release acceptance is claimed.
The 25-file, eight-archive checkpoint, including 53 raw Rust inputs, is retained
under `artifacts/media-verification/2026-09-12-d3-metadata/`. Completed worker
worktrees, disposable gate databases/volumes and managed test media were removed.

## Mutating Helper Order (2026-09-12)

The next D3 increment runs the real conflict-writing helper before ingestion
in the same cold backend, with separate committed and whole-rollback cases.
It covers both 256-character truncations and the default observation clock;
independent models retain all 18 table images and the three consumed conflict,
audit and health sequences. Existing pure-helper orders and D4 failures remain.
Initial focused suites pass 13,474 assertions, including 456 additional
mutations rejected. Fresh database and combined gates are pending. This is
proof-only work with existing dependencies and no SQL/runtime/init delta;
rollback removes this increment. Root, data, Rust, DevOps and Sonar instructions
were reviewed; DevOps now requires the actual same-backend mutating controls.
No broader D3, helper/native closure, cutover or new approval is claimed.

The bounded PostgreSQL run at clean `119ec599` passes 261 checks, including
24 metadata variants across three cases and four compilation modes. The
12 paired outcomes retain the exact D4 exception; `d3_complete=false` remains.
Its original report, raw evidence and real Ruby coverage are archived under
`artifacts/media-verification/2026-09-12-helper-order/`; the exact disposable
container and named volume were removed and absence verified. Independent
two-file review reports no actionable finding after replaying the 24 variants;
its temporary worktree was removed. This is not a full-service or Sonar pass.

A further wrapper regression distinguishes two actual size samples (100, 900)
from their median (500): the canonical display must remain the first sample
(100) until the third sample. The existing cold, helper-first and same-backend
rollback/retry modes exercise it without changing stored procedures. Separate
checks reject wrong rollup counts, extrema, median and premature promotion.
This is the same proof-only implementation boundary with no new dependency;
DevOps now records the two-sample requirement. Full canonical and combined
gate results remain pending and no existing acceptance condition is removed.

## Size And Domain Boundaries (2026-09-12)

This proof-only increment adds 19 independent sampling decisions: below, at and
above the exact 10 TiB cutoff; requested versus effective domain; no, single and
multiple instance domains; each ebooks/audiobooks/software exception; zero/NULL;
and normal/large title fallback without sampling. Each runs through the real
application wrapper cold, helper-first and with whole rollback/retry. Expected
values cover all 18 write tables and eight unchanged read tables, including
seed clocks, public identities and sequence gaps. Direct fixture seeding proves
these ingestion states, not their reachability through request-creation APIs.

Focused suites pass 14,378 assertions before the current-run consumer update.
Mutation controls reject altered full-column images, partial rollback, wrong
domains, duplicate JSON, impossible clocks and float-valued identities. Wrapper
evidence now uses unique directories; the native-dependency consumer follows
the producing run and still requires its validated byte registry. PostgreSQL
and combined gates for this increment remain pending; D3 stays incomplete.

No stored procedure, init authority, runtime policy, telemetry or approval
changes. This follows the existing proof-owner decomposition using Ruby standard
libraries, with no new dependency. Rollback removes this increment only. Root,
data, Rust, DevOps and Sonar instructions were reviewed; DevOps now records the
boundary and evidence-retention requirements. No criteria were relaxed.

The first live run at `5662cc68` rejected the model's integer zero scores:
the frozen schema declares all three scores `NUMERIC(12,4)`, serialized as
decimals. Its interrupted original run, shared correction outputs and cleanup
receipts are retained before retry. The model now expects exactly decimal
zero while still rejecting float-valued identities. This corrects the model,
not the database or comparison strictness. The disposable container and named
volume were removed; this failed run is not passing evidence.

At integrated `ccf6f7ff`, the bounded live run passes 1,422 checks across 114
variants; the canonical run passes 6,024/6,025 checks, failing only the existing
incomplete-D3 guard. Its 156 paired wrapper cases pass 3,235 checks. Containers
and the bounded named volumes were independently confirmed absent. Full CI
exits zero with the existing watcher WARNs; this is not a clean handoff.

Post-run inspection found a narrower validator defect: Ruby numeric equality
accepted a raw domain ID of `1.0` alongside a decoded ID of `1`. The current
comparison now requires the same exact table encoding for raw and declared
read inputs, and explicit regressions retain that original false acceptance.
Database results and earlier evidence remain unchanged; final-source focused
and combined verification is pending. No SQL, runtime or criterion changed.

Final executable checkpoint `be1a2a53` passes 812 focused size assertions and
all 114 retained size variants under the stricter validator. That replay is
not fresh database execution. Its subsequent canonical run independently
passes 6,024/6,025 checks and again fails only incomplete D3; all 156 paired
wrapper cases pass. Full CI exits zero with eight existing watcher WARNs.
Full UI/API has 57 passed, one schedule-PATCH failure and 72 not run; teardown
also rejects missing UI coverage. Neither required handoff gate is clean.

Local Rust LCOV has 122,900 covered DA records of 131,765 in 311 tracked
first-party files. Seven other records are retained, not counted as authored
Rust. LF/LH summary fields differ from DA totals in 251 records; the original
export and a separate comparison are retained without rewriting either. These
are actual DA counts, not a published Sonar metric or scanner acceptance.
The canonical token remains absent and the rejected upload was not retried.

Original failures, both canonical runs, real coverage and raw instrumentation
are retained at `artifacts/media-verification/2026-09-12-size-config/` with the
source bundle and exact gate driver. Containers and the UI listener were
confirmed absent; managed test-media cleanup passed. Frozen migration authority,
init cutover conditions, package qualification and PR acceptance are unchanged.

### Prevent-Merge Reachability Increment (In Progress)

The next bounded D3 family exercises all three hash strategies and both
orientations of a persisted `prevent_merge` rule through the application wrapper.
Each case first creates and independently verifies a successful no-rule fixture,
then executes two failed calls on one backend, in cold and helper-first modes.
The independently specified outcome is the frozen unique-key error, not a
successful canonical split. Every affected table, all 19 read tables, rule
identity and clock, canonical identity sequence, caller settings and exact
PostgreSQL diagnostic remain checked. PostgreSQL 16.14's independently retrieved
`nbtinsert.c` pins `_bt_check_unique` to line 666 and SHA-256
`3babaf9404d5f93dc20a5aea3dfdfffb0be636bf9ab1794c2fbb3b0bc92fe79e`.

This adds proof only. It does not repair the legacy behavior, establish rule API
reachability, cover concurrent identity races, observe every sequence or close
native/helper execution scope. The conditional-D3 guard remains incomplete.
There is no dependency, runtime SQL, observability or acceptance-criteria change;
rollback removes this proof family without changing the frozen authority.
Reviewed root instructions, DevOps, ADR 569 and the existing proof owners; the
DevOps specialization now names this narrow evidence boundary. No completion
or additional operator approval is asserted.

Focused validation passes 235 assertions. The first real run rejected integer
trust-weight expectations: `0012` declares `NUMERIC(12,4)`, so the independent
shared read model now retains decimal weights. The original failed source and
database evidence were archived before retry; no SQL or comparison was relaxed.
The subsequent pinned live run passes all 24 variants, each with one successful
fixture and two exact `23505` failures, plus the mandatory D4/D5 controls.
Both disposable runs report resource cleanup. Canonical database verification
at `c9a25f31` passes 6,049/6,050 checks; only the unchanged incomplete-D3 guard
fails. All 37 recorded proof inputs match integrated `749ba0b4`. Its full CI
fails the native deadline/pipe-closure assertion and retains four watcher WARNs;
full UI remains 57 passed, one schedule-PATCH failure and 72 not run, with
missing UI coverage rejected. Rust coverage was not reached by this CI run.
See [combined gates](586-compliance-manifest-failure-boundary.md#combined-configuration-and-disambiguation-gates-2026-09-13)
for exact limits and retained evidence. No native deadline or test criterion was
changed. Frozen authority and conditional D3 remain in force.

## Native Callback Counter Limitation (2026-09-12)

Parent delegated one bounded function-counter investigation on `ac094da9`
under the existing isolated-investigation goal. This allocation was not a new
decision-specific human approval and did not authorize another profiler or
runtime design. PostgreSQL 16.14's
[function manager](https://raw.githubusercontent.com/postgres/postgres/REL_16_14/src/backend/utils/fmgr/fmgr.c)
sets built-in/internal functions to the never-track threshold before callback
dispatch. The pinned
[statistics implementation](https://raw.githubusercontent.com/postgres/postgres/REL_16_14/src/backend/utils/activity/pgstat_function.c)
therefore creates no counter for them, even with `track_functions=all`.
The exact low-level
[transaction counter API](https://raw.githubusercontent.com/postgres/postgres/REL_16_14/src/backend/utils/adt/pgstatfuncs.c)
returns NULL for a missing entry. Bypassing the user-function view cannot
make built-in RI callbacks observable, and NULL must not become zero calls.
The [trigger caller](https://raw.githubusercontent.com/postgres/postgres/REL_16_14/src/backend/commands/trigger.c)
also updates usage after normal return, not as an error-surviving entry trace.

The dependency owner now rejects shared reference/final substitutions of RI
language, native symbol, arguments, result type, library, settings and security
mode. Its installed/eligible callback records still explicitly deny execution
proof. Focused unit coverage adds eight substitution rejections and an absent-
counter limitation check; the 208-assertion suite passes. No new dependency,
observer trigger, runtime telemetry, production/frozen/init SQL, normalization,
timer, permanent privilege or approval change is introduced. Rollback removes
this guard and limitation record only; the incomplete-D3 sentinel remains.

The retained reproduction is under
`artifacts/media-verification/2026-09-12-d3-callback-counter/`, including pinned
upstream source bytes, source deltas and individually numbered attempts. It
uses the existing uniquely owned network-none bootstrap, required D4/D5 matrix
and the wrapper producer's independently validated bytes and source hashes.
The exact frozen candidate is recovered from `7fd5df30` and verified against
the unchanged candidate pin before use. Original import, sandbox and probe-
framing failures are retained separately, including shared correction outputs
before retry; none is relabeled as native evidence.

Attempt 04 passes 306 bounded checks. The first-visible application wrapper
runs cold/helper-first, reference/final and counters disabled/enabled: eight
validated runs with complete 18-table images, actual results and unchanged
caller compilation settings. Every profiled C `digest(bytea,text)` control
increments by exactly one; all three sampled internal RI counters remain
NULL before and after ingestion. Four separate privileged, isolated inserts
into the real `canonical_size_sample` table retain SQLSTATE `23503`, the named
FK and native `ri_ReportViolation` diagnostic, unchanged table images and NULL
RI counters. These actual native failures are counterexamples to observability,
not numeric callback counts or evidence for other ingestion branches. No
test-authored trigger substitutes for RI. Both created containers and named
volumes were removed; successful removal and exact-name absence receipts are
retained. This is a precise negative result for this mechanism only.

Stale-policy check: root AGENTS, data and DevOps guidance, ADR 569 and the
operator's ADR 588 approval boundary were reviewed. DevOps now records this
counter limitation; no contradictory authority was removed or relaxed. Parent
owns integration, ledgers/indexes and full CI/UI. No canonical full proof,
full gates, alternative profiler, native closure or complete D3 is claimed.

## Committed Sampling Evidence (2026-09-13)

In progress under conditional D3: add three-call application-wrapper cases for
median promotion, duplicate sample suppression and out-of-order observations.
Both cold and pure-helper-first modes retain the same tested backend across
commits. The frozen reference must keep its exact approved D4 failure on calls
two and three; the final implementation must succeed. Independent expectations
cover all 18 write tables, unchanged 19 read inputs and the sample identity
sequence, including duplicate allocation. Other sequences and complete native
or warm-path closure are not certified. The incomplete-D3 guard remains.

Mutation tests reject substituted state, raw/declared evidence divergence,
caller role/settings/backend changes, invalid clocks, numeric type changes and
altered native diagnostics. The focused suite passes 200 assertions and the
complete database unit harness passes. The pinned live run passes 237 checks,
including required D4/D5 controls and all 12 committed sampling variants, with
exact source hashes and successful disposable container/volume cleanup.
Two test-harness failures are retained before correction: the diagnostic hash
needed its already-approved `SQL statement` representation, and synthetic
readback snapshots needed independent copies. Neither fix changes a criterion
or database implementation. Integrated gates remain pending. Evidence
uses uniquely owned disposable databases and retained raw transport; no runtime
observability, dependency, SQL, privilege, timer or approval boundary changes.
Rollback removes only the added proof module and tests. Root, data and DevOps
instructions plus ADR 588 approval were reviewed; DevOps records the bounded
committed-call requirement without weakening or contradicting existing policy.

The canonical proof at `f5981931` passes 6,062/6,063 checks. Only the unchanged
incomplete-D3 gate fails. All 38 sampling-proof source hashes also match the
combined shutdown revision `fe0948a4`; its full CI passes without WARN or
compiler-warning lines. UI still fails schedule enablement and missing UI
coverage. See the [combined checkpoint](586-compliance-manifest-failure-boundary.md#sampling-and-owned-watcher-integration-2026-09-13).
Frozen migrations remain runtime authority; final init remains inert.

## Magnet Normalization Branch Evidence (2026-09-13)

In progress under the existing conditional D3 approval. Three actual-wrapper
identity inputs cover an empty magnet query, discarded empty parameter keys,
and a bare parameter without a value. Each exercises new identity, reuse and
GUID promotion in cold, helper-first and same-backend warm-after-rollback
sessions. Expected normalized URI hashes come from independently specified
text; all prior result, identity, read-input and 18-table assertions remain.
Direct helper-first known answers retain all earlier inputs and add these
three; mutation checks reject every substituted normalization output.

This is proof-only work: no database SQL, runtime, privilege, dependency or
approval change. Observability consists of retained local SQL, diagnostics,
source hashes and disposable resource cleanup receipts. Live and integrated
validation are recorded below. These cases do not close concurrent GUID conflicts,
native trigger execution or D3 overall. Rollback removes this proof increment
only, retaining frozen authority and the incomplete-D3 failure. Root, data and
DevOps instructions and ADRs 569/588 were reviewed; DevOps now records this
bounded requirement without weakening existing criteria.

At `36abef44` plus this six-file executable proof/test delta, the focused
ingestion suite passes 4,746 assertions and every other database proof family
passes. The rebaseline unit harness passes 45 assertions. The pinned live
run passes 738 checks, including 27 reference/final pairs (54 new variants)
and the separate mandatory D4/D5 controls. All recorded source hashes still
match after execution. The uniquely owned container and named volume were
removed. No first-attempt database failure occurred in this increment.
Canonical proof, full CI/UI and published Sonar evidence remain pending.
