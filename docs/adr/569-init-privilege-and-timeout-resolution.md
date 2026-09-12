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
