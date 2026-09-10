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
