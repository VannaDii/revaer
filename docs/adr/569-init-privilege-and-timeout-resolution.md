# Init privilege and timeout resolution

- Status: Proposed
- Date: 2026-09-10
- Operator approval: Pending. Approval of ADRs 557-559 does not approve this delta.
- Context: ADR 566's real PostgreSQL 16.14 proof found two incompatibilities
  between the frozen candidate and ADR 551's exact finalization requirements.
- Decision: None made. Present D1 and D2 below for separate operator approval.
  Keep the finalization proof failing and the runtime cutover disabled meanwhile.
- Consequences: These proposals preserve a constrained bootstrap principal and
  stored-procedure-only runtime access, but alter exact accepted schema/parity
  and timeout-scope constraints. They are not G1 internal refinements.
- Follow-up: Obtain an explicit decision, then implement only the accepted
  delta, regenerate its exact init digest, and repeat all conformance gates.

## Observed Conflicts

The source for this review is the inert finalization in commit `b74d6b1a`
(worker commit `d451169c`). Its final-init SHA-256 is
`d27d2d99a0957b2502d0461b27c29ed1f2a53e486d1a140e7aff8e5429d7f069`.
The proof completed 64 checks: 62 passed and these two failed. This digest is
not a release certification and has not been embedded or deployed.

1. **Extension execution.** The constrained owner installs trusted `pgcrypto`
   and `unaccent`, but 40 extension routines belong to PostgreSQL's bootstrap
   superuser. They retain PUBLIC execution in `public`, which the runtime must
   be able to use for application procedures. Omitting explicit runtime grants
   does not remove that effective permission. The constrained owner cannot
   revoke privileges on those superuser-owned routines. This is PostgreSQL's
   documented [trusted-extension ownership behavior](https://www.postgresql.org/docs/16/sql-createextension.html),
   not evidence that Revaer needs an unrestricted bootstrap account.
2. **Timeout leakage.** The seed path calls
   `revaer_config.factory_reset_without_media_defaults_v1`, whose existing body
   executes `set_config('lock_timeout', '5s', true)`. It leaves the encompassing
   transaction at `statement_timeout=2min`, `lock_timeout=5s`, and
   `idle_in_transaction_session_timeout=30s`, instead of `2min/2min/30s`.
   Preserving that body unchanged conflicts with the caller-owned timeout rule.

The proof already rejects both outcomes. No success predicate, role privilege,
extension ACL, timeout value, migration, workflow, or remote setting was changed
to conceal them. The independently passing pristine catalog and baseline-reader
tests do not resolve either conflict.

## D1: Private Extension Schema

**Recommendation, not authorization:** add exactly one private schema,
`revaer_extensions`, owned by the constrained schema owner. Install both trusted
extensions there inside the same authoritative init transaction. Revoke schema
USAGE and CREATE from PUBLIC and the runtime; grant neither to runtime. Keep
the extension member objects owned as PostgreSQL installs them, without granting
superuser or role-administration privileges to Revaer.

- Qualify every authored extension reference as `revaer_extensions.<object>`.
  Include extension functions, dictionaries, and any dependency discovered by
  catalog/definition inspection, not just currently exercised function calls.
  Keep application routine search paths within ADR 551's existing fixed set;
  do not add the private schema to runtime or application search paths.
- Only owner-executed application SECURITY DEFINER routines may reach these
  private dependencies. Runtime direct extension calls and all extension DDL
  must fail, including attempted name, cast, prepared-statement, indirect-role,
  and object-identity access paths available to the constrained runtime.
- This changes ADR 551's literal extension-schema parity and its effective
  routine-admission test. Extension function ACLs can still contain PUBLIC
  EXECUTE, but schema access must deny direct runtime invocation. A bare
  `has_function_privilege(..., 'EXECUTE')` result is not sufficient evidence of
  invocation permission or denial. This distinction requires explicit approval;
  it is not an already-approved reinterpretation of the failing check.
- Keep exact extension versions, member definitions, and dependency identity
  parity, with only the approved schema/reference relocation delta. Record and
  validate every changed object, including seed behavior. No dynamic schema
  rewriting, provider allowlist, second SQL authority, or new dependency.

This is a proposed permanent v0 architecture change, not a temporary Sonar,
security-advisory, required-check, or coverage exception. It does not authorize
changing any such criterion. If the operator requires literal revocation of
PUBLIC EXECUTE on every extension member, D1 does not satisfy that requirement;
the operator must reject D1 or approve a separately designed administrative
provisioning model. Do not silently swap those guarantees.

Alternatives considered:

- Keep extensions in `public` and accept their direct runtime execution:
  smaller change, but relaxes the intended runtime boundary; not recommended.
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

## Acceptance Evidence Required After Approval

- Fresh init under the pinned constrained owner; direct owner/runtime/outsider
  sessions; no role substitution; no added role or server capability.
- For D1, no runtime/PUBLIC schema access or direct extension call, including
  after owner NOLOGIN. Execute every affected application procedure path through
  the runtime role, compare normalized objects/seeds, and prove transaction
  rollback leaves no private schema or partial extension installation.
- For D2, prove exact before/inside/after values for successful reset, lock
  contention, SQL failure, cancellation, transaction rollback, and nested calls.
  Preserve the fixed whole-operation deadline and final cancellation reason.
- Mutation tests must reject runtime USAGE, public relocation, unqualified
  extension references, broadened search paths, timeout leakage, and removal or
  increase of the scoped bound. Existing failing proof assertions remain until
  the operator accepts the exact replacement contract and tests.
- Run full application parity, `just ci`, `just ui-e2e`, positive published
  Sonar coverage and applicable GitHub checks on the resulting stack revisions.
  Freeze/embed no final digest and enable no cutover before all required proof.

## Task Record

- Motivation: Present one evidence-backed approval delta for the two real
  single-init conflicts instead of inventing architectural consent or relaxing
  a failing gate.
- Design notes: Recommendations above identify exact predecessor constraints,
  changes, alternatives, and retained boundaries. No prototype or production
  implementation of D1 or D2 is part of this record.
- Test coverage summary: Source evidence is ADR 566's completed 62/64 live
  proof, not a test of either proposal. Proposed acceptance cases are not passes.
- Observability updates: Retain the two named failures and bounded evidence;
  no runtime telemetry, error contract, or credential handling changes.
- Status-doc validation: Reviewed the completion goal and ADRs 522, 541, 551,
  and 559. Single-init remains inert; E1 and other retained holds remain held.
- Risk & rollback plan: D1 can break extension references or admit unexpected
  indirect access; D2 can change nested reset timing. Both remain unapplied
  pending approval and proof. Rejecting this ADR leaves current runtime and
  GitHub unchanged; never repair a sealed baseline in place.
- Dependency rationale: No new dependency proposed. Keep the existing pinned
  PostgreSQL tools, extensions, SQLx transport, and canonical recipe surface.
- Stale-policy check: Reviewed root, data, Rust, devops, and Sonar instructions.
  No instruction, accepted ADR, required check, quality criterion, or frozen
  migration is changed by this proposal. Indexes and generated docs are updated.
