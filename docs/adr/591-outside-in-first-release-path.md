# Outside-in first-release execution path

> Current recovery direction: [ADR 593](593-simple-checkpoint-recovery.md)
> supersedes ADR 588's custom owner/controller machinery and ADR 589's handshake.
> Historical implementation checkpoints are not current implementation mandates.
> Other approved contracts remain binding except where explicitly superseded.

- Status: Accepted
- Date: 2026-09-15
- Operator approval: 2026-09-15, "ADR 591 is approved."
- Supersedes: Only the pre-cutover feature-schema prohibition and legacy-parity
  release prerequisite in ADRs 522/551, under approved decision 1.
- Implementation status: Approval and instruction alignment recorded; root
  readiness API and profile request implementation in progress under 557/590.
  Persisted workflow, single-init cutover and release qualification remain outstanding.

## Current Operator Goal, 2026-09-21

Complete and merge the full first-release media transcoding service under
`MEDIA_TRANSCODING.md` and approved ADRs, single-agent and outside-in, with
minimal token usage. First save configuration through the authenticated UI,
restart the real service, and complete a dry-run plan with proof that source
media is unchanged. Then finish execution, verification, safe replacement and
recovery; this milestone does not reduce release scope.

Use focused searches and targeted tests during implementation. Reuse verified
evidence unless relevant inputs change. Avoid broad audits, repeated reporting,
unrelated database investigations and non-actionable polling. Maintain a short
checkpoint of completed work, the current failure and the next action.

Completion requires passing `just ci`, `just ui-e2e`, strict Sonar with positive
coverage, applicable GitHub checks, resolved PR feedback, and validated Linux
amd64/arm64 packages. Deliver through one linear reviewable PR stack, honoring
the canonical changed-line ceiling and never exceeding 10,000 changed lines.

Never weaken gates, invent architectural approval, drop features or pursue
migration/backward-compatibility work. Preserve user changes, remove completed
worktrees and delete test media after every turn. Report blockers once with the
exact approval needed. Investigation and partial tests are not completion.

This operator direction supersedes the old goal's investigation-first and
parallel-agent execution instructions; it does not expand architecture or
external-transfer approvals. The app goal remains usage-limited and its stored
objective has not been replaced by this task-record update.

## Approval Resolution

On 2026-09-15 the operator approved ADR 591 and separately approved ADRs 590
and 589 under the presented recommendations. The operator further directed:
"This is an unreleased product" and "Stop considering migrations and backward
compatibility and legacy anything." Non-development use before release is at
the user's own risk; no compatibility or upgrade obligation is assumed.

All three decision holds below are released. Do not restart migration,
backward-compatibility or legacy-equivalence investigations as feature or
release prerequisites. Existing historical artifacts are not authority to
reintroduce those obligations. Preserve user work and data; this approval does
not authorize destructive resets of caller-owned databases or weaken media
safety, security, CI/UI, Sonar, package or review requirements.

The first delivery milestone is real catalog reads, a saved versioned profile
and a saved association through the authenticated application against the
single-init candidate. Database changes serve that workflow. Approval is not
implementation, qualification or merge evidence.

## Decisions Requested

1. **Allow real feature development against the single-init candidate now.**
   Replace legacy-equivalence qualification as the v0 cutover prerequisite with
   fresh-install, security, transaction and complete application/workflow
   qualification below. Permit operator-approved media schema/procedure changes
   in that candidate before cutover. I recommend approval.
2. **Approve [590](590-profile-version-wire-completion.md)** for the complete
   profile payload, exact version references, enabled-state behavior and safe
   authoring defaults. This closes the profile editor's public-contract gap.
3. **Approve [589](589-hash-helper-registration-deadline.md)** to carry the
   already-running effective fingerprint deadline in `REGISTER_HASH`. It adds
   no duration and permits no clock reset.

These are the known pending decisions, not a request to reapprove ADR 588.
Decision 1 replaces a validation requirement; it is not an internal refinement
or an implied waiver. Decisions 2 and 3 remain independently reviewable.

## Why Decision 1 Is Needed

The current data instructions forbid feature schema changes until cutover.
ADR 551 requires legacy parity before that cutover, allowing only enumerated
finalization changes. The approved catalog readers do not exist in the frozen
migrations or data/runtime adapters. Consequently, approved catalog UI cannot
read a real persisted generation without first completing the old transition.
Continuing to add draft UI does not remove that prerequisite.

The tracked init file already exists; this is not a proposal to start a new
database system. The product is unreleased, and ADR 522 already disallows
in-place database upgrades without a separately approved data-preservation plan.

## Exact Replacement Boundary

- Keep the frozen migration corpus immutable as reference evidence. Stop making
  new-feature implementation wait for byte/catalog equivalence to that corpus.
  Author only approved feature semantics and behavior-preserving bug fixes in
  the single `crates/revaer-data/init.sql` candidate; no new migration files.
- Permit an explicitly selected, owned disposable database initialized from
  the complete candidate for real API/UI testing. It must use the actual
  application, authentication, stored-procedure callers and constrained runtime
  role. No production test flag, fake repository, permissive role or alternate
  media engine is authorized. Existing caller-owned databases are never reset.
- Do not carry unqualified D3 compiler substitutions into the selected
  candidate. Preserve that work as historical evidence. Fix an ambiguous
  routine through a narrowly reviewed correction with real caller, failure and
  rollback tests; do not resume exhaustive cold/warm legacy-equivalence work
  merely to implement a new media endpoint.
- Before atomic runtime cutover, require fresh installation from exact init
  bytes; baseline seal/digest checks; least-privilege and denied-operation tests;
  transaction rollback and failure recovery; the complete existing application
  suites against init; and real configuration-to-recovery media workflows with
  dry-run/original preservation. Retain ADR 551's package, PostgreSQL, ownership,
  timeout, idempotency and failure contracts. D1/D2/D4/D5 are not relaxed.
- Keep `just ci`, `just ui-e2e`, positive published Sonar coverage, all applicable
  GitHub checks, review resolution and native Linux amd64/arm64 qualification.
  Do not change required-check names, triggers, rulesets or Sonar criteria.
  Replace only the parity-specific transition acceptance; update its recipes,
  guards and scoped instructions together after explicit approval.
- Keep the linear PR stack and canonical line ceiling. Merge and retire inert
  migrations/transition tooling in bounded slices. This authorization ends with
  the first media v0 release; it does not authorize upgrades of existing data or
  a post-v1 migration system.

## Tradeoff And Rollback

This removes exact legacy-equivalence assurance. Full functional/security tests
can still miss unexercised regressions; neither a green suite nor this approval
proves perfection. Keeping the existing rule retains that assurance target but
keeps the feature path dependent on the transition proof. A second permanent
schema or a production bypass would create a different system and is rejected.

Before cutover, rollback means reverting candidate changes and deleting only
owned disposable databases. After cutover, use the approved package/baseline
recovery contract; do not rewrite a sealed baseline or destroy operator data.

## Task Record

- Motivation/design: expose the actual prerequisite and replace repeated
  infrastructure investigation with a testable outside-in delivery path.
- Test evidence: bounded source inspection of ADRs 522/551/557, data instructions,
  frozen migrations, data/runtime readers and canonical database recipes. No
  candidate experiment or runtime change was executed for this proposal.
- Observability: retain exact-revision gate results and report implemented,
  locally verified, package-qualified and merged states separately.
- Dependencies: none proposed. No new language, database system or tool fleet.
- Stale-policy check: root, Rust and data instructions reviewed. The freeze is
  an active approved constraint, not silently dismissed as stale. Its exact
  replacement requires operator consent; existing user work remains untouched.

### Approval Recording, 2026-09-15

- Recorded explicit approval in 589-591 and the register/navigation. Root and
  data/DevOps instructions now identify the approved replacement of historical
  transition prerequisites. Historical work is preserved, not resumed.
- Reviewed root and data/DevOps instructions; removed the conflicting root
  cutover restriction and explicitly scoped superseded transition instructions.
  No workflow, SQL, runtime or Sonar setting changed in this approval update.
- Stored the operator-requested unreleased-v0 rule in persistent memory.
- Approval-record validation: `just docs-index` and `git diff --check` passed.
  `just instruction-drift` returned zero but emitted Xcode-license errors from
  its nested Git calls; this is not a valid pass. No license was accepted on the
  operator's behalf. Full CI/UI gates were not rerun for this approval record;
  no test media was acquired or generated.
- The evidence below predates this documentation update and does not certify
  the new revision or the approved implementation.

## Current Validation

### Single-Agent Milestone Resumption, 2026-09-21 (Incomplete)

- Profile-create boundary: authenticated POST now requires exactly one wildcard
  `If-None-Match` before JSON decoding. Missing headers return 428; malformed
  headers return 400, using bounded problem JSON and no-store. Router tests
  preserve authentication precedence. All 70 scoped API media tests and strict
  all-feature API Clippy passed; generated OpenAPI/client and TypeScript coverage
  compilation are updated. Existing success assertions were not weakened.
  This is not immutable creation: legacy request/storage semantics remain to be
  replaced, including complete versioned target/policy
  references, root bindings and response ETags. No end-to-end save is claimed.
- Duplicate-create correction: the existing init already rejects duplicate keys
  atomically; the application misclassified that result as storage failure.
  It now maps to HTTP 409. Application/API regressions and scoped app Clippy
  pass. The real API-key test at `target/milestone-profile-api-1790022380-61638`
  passed create/conflict/readback/source-byte preservation. Its command still
  failed the unchanged global API coverage gate (142 entries absent).
  This exercises the legacy create surface, not immutable profile authoring,
  UI/restart/planning or transcoding. No schema or release gate was changed.
- Current checkpoint: the authenticated root-catalog listing now reads the
  actual persisted catalog through ADR 557's bounded slot-page procedure.
  Metadata and rows share one statement snapshot; the response validates
  generation coherence, ordered kinds, evidence and continuation before return.
  The real HTTP empty/missing-state and invalid-limit tests pass. Restricted
  runtime tests pass for populated synthetic pages, allowed-kind grouping,
  continuation, invalid cursors and reset. Synthetic rows do not prove real
  filesystem attestation. Unit authentication/conversion tests and strict
  scoped Clippy pass. No new dependency or architectural decision.
- Current failure: full `just ui-e2e` at
  `target/milestone-ui-e2e-live-1790014498-74121` passed 61 API tests and failed
  scheduled-profile setup with `media_profile_filesystem_identity_required`;
  84 dependent tests did not run and UI coverage was absent. The safety check
  and success expectation remain unchanged. This is not authenticated UI
  save/restart/dry-run milestone evidence. Earlier CI and release blockers
  remain unresolved; no push or merge is claimed.
  Fresh `just ci` at `target/milestone-ci-1790014697-83149` stops at the same
  historical candidate digest/statement-count assertion in
  `scripts/tests/database-final-test.rb`; that gate was not relaxed or skipped.
- Runtime checkpoint: the opened-root handle now retains every no-symlink
  directory link, validates ownership/permissions and the injected service UID,
  holds every declared exclusive descriptor lock (including source-only), and revalidates links
  before/after the capability probes. Source-only handles cannot invoke write
  probes. Read-only probes exercise descriptor-relative traversal and directory
  enumeration without creating entries; they do not assert descendant access.
  Source preservation, replaced-root rejection and denied search permission are
  tested. A separate test process was denied while the lock was held and
  admitted after release. Replaced roots and unsafe ancestry fail before writes.
  Linux revalidation now requires reported, bigint-bounded `statx` mount IDs
  on retained descriptors and current parent links, not just device/inode.
  Missing or changed IDs fail closed. All 93 root tests passed on the local
  Linux arm64 kernel, including descriptor-derived mount IDs, process locks,
  source loading and filesystem probes. Strict Linux arm64 Clippy passed.
  Loaded catalogs now retain their source file and protected directory
  descriptors across parsing and cloning. Revalidation rejects post-load
  file/content/ancestor changes; a missing snapshot never silently reloads.
  Synthetic encoding fixtures cannot pass descriptor revalidation.
  Catalog-wide read probes now require fresh injected mount observations before
  and after probing. Failure at either observation is propagated; the Linux
  regression confirms no source writes. This is not deployment proof.
  This is not Linux package qualification or bind-mount replacement evidence.
  Retained handles now expose read-only descriptor-derived identity observations
  in catalog order. An injected Linux mount source reads the current process
  namespace on each call. Kernel tests compare device/inode/mount/owner/mode,
  preserve the source file and reject observation after root replacement.
  Observation debug output is redacted; these values grant no capabilities,
  writer-control or durability. No startup attestation or new operator workflow
  is complete yet: mount and
  deployment evidence remain separate, required proofs. The public platform
  boundary remains Linux amd64/arm64; host tests exercise descriptor mechanics
  without authorizing macOS attestation. Fixtures were explicitly removed.
  No new dependencies; failures remain path-free and cleanup errors observable.
  Rollback removes only the unqualified descriptor/probe implementation, never
  operator data. Reviewed existing Rust policy and the approved root contracts;
  no criteria changed. Secrets scanning passed; unavailable quality analysis
  was not retried.
- Kernel test reproduction: Rust 1.96.0 built the runtime library test binary
  for `aarch64-unknown-linux-musl` using bundled rust-lld with stable
  `-C linker-flavor=ld.lld`; GNU arm64 also passed all-target compilation.
  Existing local runtime image
  `sha256:32ee40bbb290dacbf154384bd554d643e8d25f689e1f733c23933b0bf43f15fe`
  ran `root_catalog:: --test-threads=1` with network disabled, read-only root,
  all capabilities dropped, and private `/proof` and `/tmp` tmpfs mounts.
  `HOME=/proof` satisfies the existing trusted-source fixture contract; the
  initial read-only `/root` fixture failure was corrected without test changes.
  The container and fixtures were removed on exit. Its older Rust image tag
  supplied only the Linux runtime, not the compiler or a release package.
- Catalog-handle checkpoint: all root descriptors and lifetime locks can now
  be retained together. Catalog revalidation rejects observed equal and nested
  device/inode ancestry across distinct slots without capability writes; sibling
  directories are accepted. A failed catalog open releases already acquired
  locks. The 78 root-catalog component tests and strict runtime Clippy passed
  on Linux arm64. This does not prove complete bind-mount alias topology,
  deployment ownership, durability, profile persistence or the operator milestone.
  No dependency, approval boundary or gate changed; owned container fixtures
  were removed on exit. Rollback removes only the unqualified handle code.
- Mount-observation checkpoint: added injected Linux mountinfo parsing and
  descriptor-matched device/mount/path resolution. Direct bind-root aliases
  are compared by filesystem-relative path, including ancestor aliases, rather
  than visible mount-point strings. Malformed records, duplicate mount IDs,
  absent/mismatched observations and contradictory filesystem types fail closed;
  debug/errors omit observed paths. The kernel record layout follows
  <https://man7.org/linux/man-pages/man5/proc_pid_mountinfo.5.html>.
  All 86 root-catalog component tests passed on Linux arm64, including one
  live kernel snapshot; bind-alias examples are synthetic parser fixtures,
  not privileged bind-mount or durability qualification. Strict Linux Clippy
  passed. Catalog opening and revalidation now require the supplied mount
  snapshot; alias checking cannot be omitted by a caller. Descendant mount
  regions participate across slots, including covered records conservatively,
  so a nested mount cannot hide a cross-slot alias. Native bind-mount race
  qualification and snapshot freshness still require independent evidence.
  No new dependency or readiness grant; deployment evidence, startup
  attestation and profile persistence remain unfinished.
  Scoped profile-save inspection confirmed the legacy handler and incomplete
  policy aggregate cannot implement ADR 590 through a compatibility adapter;
  no default policy fields or placeholder paths were introduced.
- Startup-persistence checkpoint: implemented ADR 557's exact two fail-closed
  state writers and typed bootstrap-only callers. Each caller uses a serializable
  transaction; the procedures share the approved reconciliation lock, clear the
  active generation, keep history, accept only closed state/reason pairs, and
  retain fixed-search-path definer execution with PUBLIC revoked. Restricted
  runtime tests exercise invalid inputs, rollback, all failure states, hidden
  invalidated catalog rows and denied direct table reads against the full init.
  The focused baseline suite and strict data Clippy pass. This is not startup
  reconciliation or workflow completion; no HTTP mutation route was added.
  The initial owned PostgreSQL host-bind fixture did not become ready; the same
  assertions passed with disposable Linux tmpfs storage in
  `target/milestone-baseline-1790018141-56686`. No durability claim is made from
  tmpfs. Owned container/roles were cleaned; no unrelated containers were touched.
  No dependency or architectural change; reviewed the existing data policy.
  Rollback removes this unqualified code, not retained operator catalog data.
- Activation dependency: PostgreSQL now independently reconstructs source,
  slot, aggregate-attestation and generation digests from normalized rows.
  Fixed Rust vectors match, including Unicode lengths, unsigned identity/owner
  boundaries, empty catalogs and reversed insertion order. Changed kind rows
  cannot reuse a prior slot digest. Helpers remain ungranted to runtime/PUBLIC;
  exact-init restricted-runtime tests and strict data Clippy passed. No ready
  generation is activated by these helpers alone. Startup attestation and the
  operator milestone remain unfinished. No new
  dependency or architectural decision; fixture changes are rolled back.
- Reconciliation checkpoint: implemented the four approved begin/slot/kind/
  activate procedures. They serialize on the catalog lock, require serializable
  transactions, independently verify digests before activation, enforce ordered
  appends and capabilities, and reject incomplete generation commits. Begin
  stamps transaction ownership without refreshing proof time; later calls cannot
  reactivate a historical generation or skip begin in their transaction. Exact
  runtime-role tests pass for populated synthetic vectors, loaded-empty state,
  idempotent reuse, missing-begin rejection, incomplete commit rejection and new
  generation identity after source loss. Evidence:
  `target/milestone-baseline-1790019069-74474`. This does not establish filesystem
  authority, startup wiring, concurrent-caller qualification or profile saving.
  Helpers and trigger functions are not granted to runtime or PUBLIC; no direct
  table privileges or dependency were added. Strict data Clippy and secrets
  scanning pass. Rollback removes unqualified code, never operator history.
- Typed reconciliation adapter: owns the serializable transaction from begin
  through ordered scalar slot/kind calls, activation and commit. Returned
  metadata is exposed only after commit; explicit rollback completes before a
  separate failure-state mutation. The actual adapter passed populated vector
  bindings, committed readback, idempotent identity/activation-time preservation
  and rejected-kind rollback against the restricted runtime fixture. Follow-up
  failure tests also prove readiness is unchanged after an abandoned adapter or
  activation rejected for missing slots/kinds/write capability or wrong digest;
  the single-connection pool remains usable after each rollback. Evidence:
  `target/milestone-baseline-1790019606-84788`. These synthetic rows are not
  filesystem proof. Startup must still retain/revalidate descriptors immediately
  before activation; the adapter itself grants no filesystem authority. No new
  dependency or gate change. Strict data Clippy and scoped secrets scanning pass.
- Next action: verify filesystem/deployment evidence using the retained
  descriptors, then connect root reconciliation/attestation to the approved
  immutable profile save, then validate that save through the UI and a restart.
  Do not repeat the unchanged scheduling failure, broaden the database task,
  invent placeholder paths or fabricate a ready catalog to unblock saving.
- Catalog validation: `sonar analyze secrets` passed on the scoped source and
  tests. `sonar analyze sqaa --file crates/revaer-app/src/media/root_catalog.rs
  --project VannaDii_Revaer` is unsupported by the installed CLI; its supported
  `sonar verify` equivalent reported Vortex unavailable. No quality scan or
  positive coverage is claimed. OpenAPI and client types were regenerated.
  Root, Rust, data and DevOps instructions were reviewed; no gate or approval
  boundary changed. Errors remain bounded/path-free and responses no-store.
  Roll back only this unqualified code delta before deployment, never a sealed
  operator database. Owned PostgreSQL fixtures were removed after validation.
- Save-path inspection confirms the server still calls the path-taking upsert;
  `media_profile_version` and version-root-binding persistence are absent.
  The existing profile parent requires source/output paths, so it cannot serve
  the approved logical-only create by inventing placeholder paths. Next perform
  the coordinated versioned-profile persistence/handler replacement under
  ADRs 521/557/590, including its real root-catalog binding dependency. Do not
  add a compatibility adapter or claim the editor already saves to storage.
- Corrected profile confirmation to require ADR 557's exact
  `"media-profile:<public-id>:v1"` ETag rather than any strong tag. The five
  focused authoring tests pass, including wrong identity, version and resource
  kind; no new architecture or API spelling was invented.
- Earlier association work: save confirmation now compares the submitted
  profile identity/version, logical source, exact relative scope and all three
  discovery modes before showing success. Seven focused association unit tests
  passed, including independent mismatches for those seven fields. The mocked
  browser success fixture now returns the submitted fields; its browser test
  was not rerun. This is frontend confirmation safety, not persisted workflow
  or restart evidence. Secrets scanning and formatting passed. Current full
  gate failures remain as recorded below; next implement the complete approved
  profile response/editor and real save endpoint without legacy aliases.
- Recovered the prior uncommitted operator-workflow implementation after its
  temporary worktree disappeared, into the durable `work/media3-operator-workflow`
  worktree based on `f4b80bf7`. Preserved the separate user checkout and conflicts.
  Recovery is not delivery, and no subagents were used for this resumption.
- Converted real-service E2E provisioning to the complete init and sealed,
  restricted runtime login. The fixture owner is disabled before service use;
  cleanup requires the captured container identity and owned database name.
  Administrative URLs must identify the same published loopback endpoint and
  cannot override runtime identity through query parameters. Malformed URL
  diagnostics do not retain credentials.
- The real authenticated readiness request exposed missing catalog seeds after
  factory reset. Moved the initial reset after root catalog declaration and
  reseeded the five missing/unverified root kinds in the canonical reset body.
  Readiness then passed in the real API suite. No ready filesystem identity is
  synthesized, and source media is not modified by this correction.
- The next real API failures were deferred job-integrity enforcement losing
  table privileges at commit and an ambiguous compatibility-target conflict
  expression. The candidate now uses the sealed owner and fixed search path
  for the deferred trigger, and qualified target columns without a compiler
  conflict directive. Added target create/update/rejected-write readback and
  runtime denied-table-access regressions. Full API qualification is pending.
- Fresh validation: `just ui-e2e-bootstrap-test` passed 35 tests plus its strict
  TypeScript check and npm audit; the three focused `baseline::pool::tests`
  passed against PostgreSQL 16.14, including exact-init verification, reset
  readiness, denied direct media-table access and uninitialized-database
  rejection. `just fmt-fix`, instruction drift and diff checks passed.
- Full `just ui-e2e` remains failing. The last inspected run
  `target/milestone-ui-e2e-1790010243-43970` passed 57 API tests, failed the two
  operations above and did not run 81 dependent tests. Its frontend coverage
  was absent. The subsequent run `milestone-ui-e2e-1790010546-61170` also returned
  failure; its generated logs remain unread pending authorization for secrets
  scanning. No UI-save/restart/real dry-run milestone pass is claimed.
- `just ci` first stopped at historical finalization byte equality. Implemented
  ADR 591's approved `feature-development` phase: require a regular nonempty
  parseable init, preserve the archived migration freeze and PR-size limits,
  and stop requiring the feature init to equal historical finalization bytes.
  All 51 static guard assertions passed. Packaged digest, sealed runtime,
  security, workflow and release qualification remain required. No historical
  parity investigation was resumed and no Sonar criterion was changed.
- The subsequent full CI run (`milestone-ci-1790011067-83792`) progressed past
  that guard and stopped in `database-final-test.rb`: its reconstruction of the
  historical candidate from current feature SQL no longer matches the frozen
  digest/statement count. That test also contains security mutations; it was
  not skipped or deleted. Its still-required safety checks must be separated
  from superseded parity assertions before CI can pass. Ordinary CI database
  provisioning also still invokes historical migrations and needs the approved
  coordinated init conversion; this run is not fresh-init CI qualification.
- Sonar secrets scans passed on the scoped repository files. Local quality
  analysis remains unavailable on this connection (Vortex unavailable); this
  is not a Sonar quality pass or positive coverage evidence. The operator
  authorized keychain access and repository-file submission; generated runtime
  log submission was separately blocked by the approval check and not bypassed.
- Observability: bounded logs remain under the worktree's ignored `target/`.
  Each validation runner removes its owned PostgreSQL container and host data
  directory on exit. No caller-owned database is reset. No remote push, merge,
  package qualification or passing GitHub-check claim accompanies this record.
- Risk/rollback: these are unqualified local implementation changes. Revert
  their scoped source changes before deployment if necessary; never rewrite a
  sealed operator database. Dependency rationale: no new dependency for these
  corrections. Stale-policy check reviewed root, Rust, data, UI and DevOps
  instructions and aligned only the superseded transition requirement.

### Read-only Runtime Bootstrap Integration, 2026-09-15 (In Progress)

- Removed configuration startup and runtime-store migration application.
  Configuration bootstrap performs read-only packaged-baseline verification,
  then injects the verified pool into I/O-free runtime-store constructors.
  The build hashes the exact init
  bytes; the verifier derives the expected login from the injected pool, admits
  only the matching sealed baseline, and detaches/closes its connection.
- Removed the three unqualified D3 compiler directives from the selected init
  as required by this ADR. This does not certify those three routines; any
  ambiguity reached by a real workflow still needs a narrow caller-tested fix.
  The historical compiler-proof work was not resumed or modified.
- Added a real PostgreSQL fixture for exact-init/seal verification through a
  restricted runtime login after the owner is set NOLOGIN, plus closed-pool and
  unmanaged-database rejection checks. All 17 focused baseline tests passed
  through `just --command cargo test --workspace --all-features baseline_ --
  --test-threads=1`, including the real init-backed runtime/readiness case.
- Dependency rationale: reuse workspace `sha2` as a build dependency to embed
  the exact init digest, and move existing `tokio` use from test-only to runtime
  for bounded acquisition/read/cleanup. No new package version or language.
- Reviewed root, Rust and data instructions. The data instructions now describe
  the read-only startup boundary. Existing tests/dev/E2E provisioning still
  need the coordinated init conversion; process cancellation and the explicit
  initialization command remain incomplete. This patch is not a runtime-cutover
  or passing-CI/UI claim and has not been pushed.
- A single bounded ADR 589 worker found the REGISTER_HASH lifecycle protocol
  prerequisite absent, made no changes and was closed. Its clean worktree was
  removed; no implementation or validation of ADR 589 is claimed.
- Evidence: `target/revaer-host-backed-baseline-f2585edcf310/gate.log`, local
  `f4b80bf7` plus this uncommitted bootstrap/baseline/fixture delta and preserved
  integration work. PostgreSQL 16.14 used the pinned image, C locale, UTF-8 and
  checksums. Exact init SHA-256:
  `b43621d8f9336bed9b475c86574b3d3f64abb522d59814a80f1df645301f179d`.
  The fixture applied and sealed those bytes in one transaction, then the
  actual pooled verifier and root reader succeeded under the restricted login.
- Validation first caught a test module-path error and an obsolete constructor
  error helper, both fixed. A later fixture cleanup failure required removal of
  the temporary owner's default privileges after reassigning its objects; the
  final run passed cleanup. Every owned container and PGDATA directory was
  removed after its run. No retained user database was changed.
- Workspace all-target/all-feature Clippy passed with the canonical warning
  policy and existing Justfile exceptions; formatting and diff checks passed.
  No full CI/UI, HTTP acceptance, initialization-CLI lifecycle, Sonar, package,
  GitHub-check or merge qualification is claimed for this delta. The next
  action is init-backed test/E2E provisioning and the authenticated readiness
  acceptance test, not legacy-equivalence research.

### Pragmatism Instruction Correction, 2026-09-21 (In Progress)

- Motivation: the operator requested durable rules preventing speculative safety
  investigations from displacing the usable workflow.
- Design: AGENTS.md section 7 is the canonical pragmatism policy. It requires
  concrete failure cases, proportional safeguards, bounded investigation,
  concise decisions, and operator-outcome evidence. Section 6 now requires
  single-agent work, removing its contradictory delegation instruction.
- Scope: instructions only; no runtime behavior, ADR 592 approval, architectural
  guarantee, or quality criterion was changed. Observability and dependencies:
  unchanged. Rollback: revert only this policy delta if the operator revises it.
- Stale-policy check: reviewed AGENTS.md and this ongoing implementation record;
  removed the delegation contradiction without duplicating rules in scoped files.
- Validation: secrets scanning and scoped whitespace checks are recorded for this
  documentation delta; full CI/UI and Sonar quality remain unqualified under the
  existing recorded blockers. No product completion is claimed.

### Profile Save Checkpoint, 2026-09-21 (Incomplete)

- Current failure: the approved complete UI profile body has no versioned
  service/persistence implementation. The current profile route remains the old
  path-taking writer. The single init has no `media_profile_version`; existing
  policy child setters still mutate policy versions. Do not bridge the new UI
  onto these mutable records or invent policy defaults.
- Corrected one directly observed save-boundary bug: the database's
  `media_policy_version_conflict` was mapped to a storage failure (HTTP 500).
  It now maps to Conflict (HTTP 409); app and API regression tests passed.
- Next implementation: complete the approved immutable configuration write
  transaction and typed service boundary required by the UI save. Root
  activation remains subject to the separate pending ADR 592 decision; no
  replacement change or synthetic readiness claim is authorized here.
- No schema change, dependency, observability change or new architecture in this
  correction. Reviewed root/Rust policy; rollback is the scoped mapping/test
  delta. Formatting, scoped strict app/API Clippy and secrets checks passed.
  Full CI/UI, Sonar quality and
  real-service milestone qualification remain outstanding.

### Root Startup Checkpoint, 2026-10-02 (Incomplete)

- Wired the existing trusted catalog source into bootstrap after sealed-baseline
  verification and before media service/worker construction. The source uses
  the approved fixed package location or exact native document-location override.
  Invalid/missing source states are persisted through the existing closed
  procedures; persistence errors prevent startup. No runtime paths, inferred
  roots, new environment content format or dependency were introduced.
- The native resolver holds source and root descriptors through service shutdown,
  probes only declared capabilities, encodes actual observations, and calls the
  ordered serializable reconciliation adapter. Source-only roots are not written.
  It skips appends for an exactly current generation and revalidates the source
  and observations immediately before activation; changed proof rolls back
  before persisting attestation failure. Source/lock owners remain live during
  that failure transition.
- This resolver currently has only disposable Linux tmpfs evidence. Other
  filesystems, persistent mounts and Kubernetes writer declarations remain
  unavailable rather than receiving unproven readiness. Those included release
  paths still require implementation and package evidence; this is not a reduced
  product scope or completion of root admission for ordinary production storage.
- Four focused tests passed in the pinned Rust 1.96 Linux arm64 container with
  read-only root, capabilities dropped and private tmpfs fixtures. They exercise
  actual source-byte preservation, write-probe cleanup, retained/released locks,
  changed source-document/root rejection, unqualified-filesystem rejection and
  closed source-error classification. They do not exercise database activation,
  authenticated profile persistence, a real service restart or dry-run planning.
- The three existing API-key readiness tests passed against the real service
  and sealed disposable database. The setup fixture resets configuration, so
  these prove empty/remediation responses and bounds, not a ready catalog. The
  focused command still failed full route coverage (142 missing entries); no
  coverage requirement was disabled. An initial incorrect platform-dependent
  expectation was removed after observing the fixture's reset semantics; the
  original assertions were preserved. Host lint, host classification tests and
  TypeScript compilation passed. Strict Linux arm64 application/runtime Clippy
  also passed after the final filesystem guard change. Full CI/UI, Sonar quality/coverage, packages,
  remote checks, review resolution and merge remain unqualified.
- Scoped secrets scanning passed for all 11 changed files with
  `just --command sonar analyze secrets crates/revaer-app/src/bootstrap.rs crates/revaer-app/src/error.rs crates/revaer-app/src/bootstrap/root_catalog.rs crates/revaer-app/src/bootstrap/root_catalog/tests.rs crates/revaer-app/src/bootstrap/root_catalog/native.rs crates/revaer-app/src/bootstrap/root_catalog/native/tests.rs crates/revaer-media-runtime/src/root_catalog/directory.rs crates/revaer-media-runtime/src/root_catalog/opened.rs crates/revaer-media-runtime/src/root_catalog/observation.rs .github/instructions/rust.instructions.md docs/adr/591-outside-in-first-release-path.md`.
  This is secrets-only evidence, not a quality-gate or coverage pass.
- Motivation: remove the demonstrated root-startup dependency of the approved
  UI save. Observability: bounded unavailable/attestation reasons and one storage
  failure origin log, with no root paths or keys in labels. Rollback: revert this
  unshipped bootstrap/probe delta without deleting source or catalog history.
  Reviewed root/Rust instructions and ADRs 550/557/590/592; updated scoped Rust
  instructions without altering approved architecture or quality criteria.
- Next action: exercise positive catalog reconciliation with the sealed runtime
  on Linux, then use the authenticated UI to save a complete profile, restart
  the service and prove a dry-run plan leaves the source unchanged. Do not count
  these component tests as that operator milestone.

### Positive Startup Evidence, 2026-10-02 (Milestone Still Incomplete)

- The actual `bootstrap::runtime_tests::e2e_serving_entry` activated a three-slot
  trusted catalog against a fresh single-init database sealed by `just
  db-test-init`. Only its restricted runtime login was supplied to the service.
  The fixture ran in the pinned Linux arm64 Rust container as UID 501/GID 20,
  with read-only root, dropped capabilities, no-new-privileges and owned private
  tmpfs roots. The serving entry substitutes compliance fixture metadata only;
  catalog loading, filesystem probes, reconciliation and service wiring are real.
- The runtime readiness procedure returned `ready/ready`, generation 1, with
  one binding-ready slot each for source, output and workspace. Backup and
  quarantine counts were zero. All destructive-ready counts remained zero;
  disposable roots did not acquire a durability grant.
- After terminating the owned process and launching the same serving entry
  again against the same files and database, generation 1 and all readiness
  rows were unchanged. Both launches reached the actual API listener. The
  source marker bytes were unchanged and output contained no probe artifacts.
  This is process-restart evidence, not graceful-shutdown, reboot, profile
  persistence, authenticated UI or media planning/execution qualification.
- The first harness stopped at its expected SIGTERM child exit (143). The
  corrected harness explicitly checked that status before restarting; it did
  not change service behavior. Media capability refresh failed on both launches
  because the compiler fixture lacks the media runtime tools; execution remained
  not ready. Do not count compiler-container evidence as package qualification.
- Owned databases `revaer_test_1790925765_24310` and
  `revaer_test_1790925851_25216` were dropped through the ownership-checking
  recipe; both application/PostgreSQL containers and private networks were
  removed. Their tmpfs source fixtures were deleted with the containers.
- Next action: authenticated complete profile save/readback through the UI,
  followed by restart and dry-run planning with real media tools. Policy
  aggregate authoring and the remaining coordinated API/UI cutover remain
  included work, not approved omissions or a new architectural hold.

### Authenticated UI Save And Restart, 2026-10-02 (Dry-Run Pending)

- Authenticated setup, desired-target creation and complete profile creation
  passed against the real Linux service using only its sealed runtime login.
  Immediate item readback and authenticated post-restart readback matched the
  saved representation exactly. The first fixture omitted the selected
  `general` v1 policy's required quarantine binding and correctly received
  `media_root_binding_incomplete`; supplying a distinct declared quarantine
  slot and binding resolved that fixture error without changing policy.
- The real browser form initially failed before POST: CORS did not permit the
  required `If-None-Match` header. Fixed the actual router to allow `PUT`,
  `If-Match` and `If-None-Match`, and expose `ETag` and `Location`. Existing
  origin handling and the exact header allowlist remain; no wildcard header
  grant, auth change, dependency or acceptance relaxation was introduced.
  The actual-router regression checks conditional POST/PUT preflights and
  exposed response headers. Host regression, strict scoped API Clippy,
  formatting and diff checks passed.
- With the rebuilt Linux service and actual compiled UI, Playwright CLI filled
  and submitted the profile form without route mocks. It confirmed version 1,
  refreshed the list, and retained disabled/dry-run-only defaults. Profile
  `46cbf525-2c55-488d-b852-c61089253df1` (`ui-milestone`) matched every submitted
  field, both heads, lifecycle and three binding-ready/non-destructive roots.
  After another process restart, authenticated item readback matched the full
  saved representation exactly and browser reload displayed the same profile.
  Source marker bytes remained unchanged. Screenshot evidence:
  `output/playwright/media-profile-live-save.png`; CLI snapshots and the
  initial CORS diagnostic are retained under `.playwright-cli/`.
- This proves real authenticated UI save and persistence across process
  restart, not a completed dry-run plan, media-file inspection, execution,
  recovery or Linux package qualification. The compiler fixture still lacks
  media tools. Background worker/recovery database errors also remain after
  entering active mode; the current path-free configuration and operational
  reader cutover are not yet complete. Browser torrent requests return 503
  because this fixture intentionally builds without libtorrent. No clean
  whole-service or full CI/UI qualification is claimed.
- Secrets-only checks passed with
  `just --command sonar analyze secrets crates/revaer-api/src/http/router.rs crates/revaer-api/src/lib.rs`.
  Sonar quality/coverage and all full release gates remain outstanding.
  Root/Rust instructions and ADRs 521/557/590 were reviewed; no policy drift
  was removed and no architectural decision changed.
- Owned test databases, containers, networks, one-use loopback credential
  transport, browser session and UI server were cleaned after verification.
  Temporary credentials remained in private disposable fixture state, never
  in repository source or reports. The screenshot contains no media fixture.
- Next action: persist the approved source association and connect discovery
  and planning to the saved active profile version, then prove a real dry-run
  plan preserves valid source media. Do not detour into unrelated database
  investigation or count the save/restart evidence as that remaining milestone.

### Association Save Checkpoint, 2026-10-02 (Positive Path Pending)

- Implemented approved immutable association parent/version storage, atomic
  serializable create/read procedures, exact runtime grants, typed adapters
  and authenticated conditional POST/item GET. Bodies and errors remain
  path-free; creation checks the exact enabled active profile version,
  shared source/output attestation, current root capabilities, component-safe
  explicit prefixes and serialized cross-profile overlap. Watcher/schedule
  activation remains closed pending its existing qualification, not removed.
  Added complete response validation, response fences and API schema exports.
- Fresh single-init installation and sealing passed against pinned PostgreSQL.
  The real Linux service passed disabled-profile rejection (409), missing
  precondition (428), invalid relative prefix (400), unqualified watcher
  rejection (409) and unknown association (404). The first unknown-item
  read exposed a SQL join ambiguity; explicit joins fixed it and the complete
  failure-path run passed. The matching Linux binary was rebuilt after the
  init digest changed; baseline checking was not bypassed.
- Profile save/readback, real service restart, unchanged catalog generation
  and original source marker bytes passed again in the same owned fixture.
  This is real service/API evidence, not a mocked frontend test. It does not
  prove a valid-media plan or positive association creation. The attempted
  shared source/output fixture was correctly rejected: its disposable root
  violates the approved restart-persistent declaration requirement. Current
  startup proof qualifies only disposable storage. Positive association
  creation, overlap/race qualification, collection/replacement/archive routes
  and manual discovery/plan integration remain unfinished.
- Model representation and HTTP response/error regressions passed (one model,
  two HTTP tests). Strict scoped all-targets Clippy, formatting and diff checks
  passed before checkpoint; generated OpenAPI was refreshed. Secrets analysis
  ran across all twelve edited source files with no findings. Quality analysis
  is unavailable on the configured Sonar connection; this is not a Sonar
  quality or positive-coverage pass. Full CI/UI, package, remote review and
  required-check qualification remain outstanding. Background discovery and
  replacement-recovery errors still leave workers paused in this fixture.
- No dependency, architectural approval or quality criteria changed. Reviewed
  root/Rust/data/UI instructions and ADRs 557/590/591; no policy drift removed.
  Existing request tracing and origin-only bounded storage diagnostics cover
  the routes; unexpected storage details are not returned to clients.
  Rollback is the unreleased-v0 code/init change with a fresh development
  database, never a migration or backward-compatibility branch.
- API-client regeneration initially failed the mandatory npm audit. Existing
  overrides pinned vulnerable `brace-expansion` and `fast-uri` releases;
  advanced them to compatible fixed versions 2.1.7/5.0.12 and 3.1.8 and
  regenerated the lockfile. The ordinary audit fix also updated existing
  transitive `@redocly/ajv` to 8.18.3. No dependency was added and no severity
  or audit exception changed. `just api-test-client` passed clean installation,
  mandatory audit with zero findings and regenerated schema output. Additional
  secrets scan passed using
  `just --command sonar analyze secrets docs/adr/591-outside-in-first-release-path.md docs/api/openapi.json tests/support/api/schema.ts tests/package.json tests/package-lock.json`.
- Sonar commands: `just --command sonar analyze secrets` with the twelve
  source files below passed. The deprecated `sonar verify` invocation on
  `crates/revaer-data/init.sql` failed because Vortex is unavailable; replaced
  the command surface, not the criteria, with the following invocation.
  Secrets ran successfully but quality was skipped for all twelve files:

  ```sh
  just --command sonar analyze --project VannaDii_Revaer --depth DEEP \
    --file crates/revaer-data/init.sql \
    --file crates/revaer-data/src/media/associations.rs \
    --file crates/revaer-data/src/media/mod.rs \
    --file crates/revaer-api-models/src/media_root_contract.rs \
    --file crates/revaer-api-models/src/media_root_contract/association_response.rs \
    --file crates/revaer-app/src/media.rs \
    --file crates/revaer-app/src/media/associations.rs \
    --file crates/revaer-api/src/app/media.rs \
    --file crates/revaer-api/src/http/router.rs \
    --file crates/revaer-api/src/http/handlers/mod.rs \
    --file crates/revaer-api/src/http/handlers/media_associations.rs \
    --file crates/revaer-api/src/openapi.rs
  ```
- All owned fixture databases, containers, networks and source markers were
  removed by cleanup traps; unrelated containers and recovered work remain
  untouched. Next action: implement and qualify the already-approved
  restart-persistent shared source/output root needed by the operator's
  association, then prove save/read/restart and a real dry-run plan on valid
  media. This is an implementation dependency, not a new approval request.

### Persistent Association Checkpoint, 2026-10-02 (Planning Pending)

- Implemented the already-approved native persistent-mount path. A fresh
  descriptor-matched namespace observation must identify an external mount
  on a device distinct from image/root placement. Native ext4 then runs the
  existing capability probes, retained ownership and final identity
  revalidation. Persisted rows now retain the declared/proven durability
  pair instead of hardcoding disposable/none. Tmpfs remains disposable;
  image-root aliases, volatile/image filesystems, other unqualified native
  filesystems and PVC declarations remain closed pending their included
  qualification. This does not prove global exclusion of external writers.
- Four native proof regressions passed on Linux arm64, including persistent
  mount classification, image-root alias rejection, source/probe preservation,
  retained locks and final source/root-change rejection. Host strict scoped
  Clippy passed. Linux Clippy initially could not run because the pinned
  compiler image omitted the component. Installed it inside an owned
  disposable compiler container, then ran the same strict scoped all-targets
  gate as UID 501; Linux arm64 Clippy passed. The serving fixture's read-only
  image, capability restrictions and runtime identity were not relaxed.
- An owned external ext4 Docker volume passed real service startup under
  UID 501, read-only image, dropped capabilities and no-new-privileges.
  The first fixture setup failed because Docker's working-directory setup
  changed the mounted root's ownership; using a separate working directory
  and disabling volume copy-up preserved the explicit fixture ownership.
  No production permission check was weakened.
- Real authenticated enabled/dry-run-only profile and association creation,
  exact item readback, overlap rejection (equal, descendant and whole-root),
  component-sibling acceptance and automatic-mode/precondition rejection
  passed. Exact complete profile and association representations remained
  identical after service-process restart and again after container restart
  with the same owned volume. Original marker bytes were initialized only
  once and remained unchanged. This is Linux volume/service/API evidence,
  not a mocked UI test, valid-media dry-run or full amd64/arm64 package proof.
- Secrets passed with
  `just --command sonar analyze secrets crates/revaer-media-runtime/src/root_catalog/mounts.rs crates/revaer-app/src/bootstrap/root_catalog/native.rs crates/revaer-app/src/bootstrap/root_catalog/native/tests.rs`.
  `just --command sonar analyze --project VannaDii_Revaer --depth DEEP --file crates/revaer-media-runtime/src/root_catalog/mounts.rs --file crates/revaer-app/src/bootstrap/root_catalog/native.rs --file crates/revaer-app/src/bootstrap/root_catalog/native/tests.rs`
  ran secrets but skipped quality because Vortex is unavailable. No positive
  coverage, full CI/UI or package qualification is claimed.
- Existing origin diagnostics report bounded root attestation failures;
  no dependency, architectural decision or acceptance gate changed. Reviewed
  root/Rust instructions and ADRs 550/557/590/591. Rollback removes the
  unreleased persistent implementation, not operator media. Owned fixture
  databases, containers, networks, volumes and marker files were cleaned;
  unrelated resources and recovered changes remain untouched.
- Next action: connect manual discovery/planning to the saved association and
  exact active profile, fixing the demonstrated operational reader failures
  only where they block that workflow, then prove a valid-media dry-run.
  Media tools are absent from this compiler fixture and discovery/recovery
  workers still report storage errors; those remain genuine unfinished work.

### Manual Preview Checkpoint, 2026-10-02 (Admission/Plan Pending)

- Replaced the discovery preview's retired profile/absolute-path request with
  an association identity and one through 128 explicit root-relative paths,
  each at most 4,096 bytes. Required named-object decoding rejects unknown,
  duplicate, positional, null and missing fields; empty, absolute, dot,
  repeated-separator and backslash paths fail. Input bytes are not trimmed or
  normalized into another candidate. The authenticated route enforces the
  1 MiB body bound, bounded 400/413 errors and no-store responses.
- Preview reads the association's exact enabled active profile and current
  binding readiness in one procedure snapshot, including that referenced
  version's dry-run policy. Relative scope comparison uses component
  boundaries; accepted source/output names remain root-relative. Existing
  count-only preview events retain the resolved profile identity. No raw
  SQL, dependency, path authority or architectural decision was added.
- The first real-service scope test returned 200 with the expected relative
  eligibility rows, but review caught missing capability-readiness admission.
  That result is superseded and is not a usable preview qualification.
  Added the existing complete-snapshot readiness gate: the tool-less fixture
  must return 503 instead of accepting operational preview. Scope eligibility
  remains only a pure model/unit result until real tools supply readiness.
- Final real Linux API verification passed after that correction: missing
  capability readiness returned 503; malformed/absolute/retired requests
  returned 400, unknown associations 404 and unauthenticated calls 401.
  Exact profile/association representations and original marker bytes
  again survived both process and container restart. All owned databases,
  containers, networks, volumes and marker files were removed afterward.
  Strict scoped host all-targets Clippy, formatting and diff checks passed.
- Model contract regression, Linux component-boundary regression, two HTTP
  invalid-list regressions and the actual-router auth/shape/body-bound/no-store
  regression passed. The router test initially lacked connection metadata
  and correctly received 401; supplying the fixture's loopback peer fixed
  the test without weakening auth. Schema export and `just api-test-client`
  regeneration passed, including mandatory audit with zero findings.
- Secrets-only analysis passed across the eleven edited source files, then
  additionally passed for `crates/revaer-api/src/lib.rs`, final app service
  code and both generated API artifacts. The same `sonar analyze --project
  VannaDii_Revaer --depth DEEP` with those eleven source `--file` arguments
  skipped quality because Vortex is unavailable. Full quality/coverage,
  CI/UI, package and remote review gates remain outstanding.
- Exact additional secrets command:
  `just --command sonar analyze secrets crates/revaer-api/src/lib.rs crates/revaer-app/src/media.rs docs/api/openapi.json tests/support/api/schema.ts`.
- Reviewed root/Rust/data instructions and ADRs 557/590/591; no criteria or
  stale policy changed. Rollback removes the unreleased preview/init delta,
  never operator source media. Current ordinary discovery/recovery readers
  and manual run admission still require the approved versioned cutover;
  retaining those unfinished paths is not compatibility work or release
  qualification. No job was queued and no real media plan was produced.
- Next action: supply the real approved media-tool fixture and connect manual
  run admission to the association's exact immutable profile/version/root
  snapshot; prove valid-media inspection and dry-run planning. Do not count
  scope matching or marker preservation as that milestone.

### Real-Tool Preview Checkpoint, 2026-10-02 (Job Admission Pending)

- The real Linux service refreshed its capability snapshot using actual
  FFmpeg, FFprobe and FFplay. Authenticated association preview returned 200
  for an in-scope relative candidate and rejected component-sibling and
  out-of-scope candidates. Invalid/retired bodies returned 400, unknown
  associations 404 and unauthenticated requests 401. Profile/association
  representations remained identical after process and container restart.
- A valid one-second FFV1 Matroska source remained SHA-256 identical across
  those restarts. This verifies source preservation and scope eligibility,
  not inspection, job admission, dry-run planning or transcoding. Discovery
  and replacement recovery still report database failures through retired
  mutable-profile readers; these are unfinished workflow dependencies.
- Diagnostic tools were Debian arm64 FFmpeg 5.1.9-0+deb12u1 in an unpublished
  disposable compiler-derived image. No repository dependency or shipping
  package pin changed. This is not Linux package qualification; the approved
  Alpine runtime and its exact package versions still require validation.
  Owned fixture databases, containers, network, volume and test media were
  removed. The local diagnostic image contains tools, not test media.
- Next action: implement approved association-based admission with exact
  immutable profile/policy versions and the five-row root snapshot, then
  exercise real inspection and a dry-run plan without changing source media.
  Do not repair the retired readers by restoring mutable absolute-path
  authority or count lexical preview as the milestone. Existing full CI/UI,
  Sonar quality/coverage, package and GitHub review gates remain outstanding.

### Planning Admission Preview Checkpoint, 2026-10-02

- Removed the planning-preview endpoint's retired mutable-profile/absolute-path
  input. It now accepts only a named association identity and one bounded
  root-relative candidate, preserves its bytes and uses the same exact-version,
  binding and capability readiness checks as discovery preview. Both routes
  share authenticated, 1 MiB bounded, no-store routing. This is eligibility
  only; neither endpoint performs inspection or produces a media plan.
- Two model regressions and the actual-router regression passed. The latter
  exercises both preview routes: authentication before malformed input,
  retired/positional body rejection, oversized-body rejection and no-store
  errors. Strict scoped all-targets Clippy passed after extracting the shared
  preview router instead of suppressing the function-size finding. API export
  and typed-client generation passed; NVM-managed npm audit found no findings.
- The rebuilt Linux arm64 real-service fixture passed planning-admission
  eligibility for `Movies/original.mkv`, component-sibling rejection, invalid
  path/retired request 400, unknown association 404 and unauthenticated 401.
  Exact saved configuration persisted across both restarts and the valid
  media SHA-256 remained unchanged. Owned fixture media, volume, databases,
  containers and network were removed. The discovery/recovery worker failures
  remain unresolved; no job or actual plan was produced.
- Exact secrets command passed with no findings:
  `just --command sonar analyze secrets crates/revaer-api-models/src/lib.rs crates/revaer-api/src/http/handlers/media.rs crates/revaer-api/src/http/router.rs crates/revaer-api/src/lib.rs crates/revaer-api/src/openapi.rs docs/api/openapi.json tests/support/api/schema.ts`.
  This is not Sonar quality/coverage evidence; the previously reported
  unavailable quality analyzer and full scanner gates remain outstanding.
- Reviewed root/Rust instructions and the accepted ADR 557 admission contract.
  No policy, dependency, approval or gate changed. Rollback removes the
  unreleased endpoint delta without touching operator media. Job admission
  and immutable five-root snapshots remain the next workflow dependency;
  do not restore mutable path authority to get the old readers passing.

### Immutable Association Admission Checkpoint, 2026-10-02

- Replaced the job-root relation with the approved ordered five-row snapshot:
  source/output/workspace are bound; backup/quarantine follow the immutable
  policy; not-required rows contain no evidence. Added restrictive identities,
  exact profile/association version references, root coherence/length bounds,
  immutable reference guards and configuration snapshot framing metadata.
  Capture no longer reads mutable profile roots or silently reselects a newer
  policy. The ungranted finalizer checks five kinds, exact copied attestations,
  profile bindings, association prefix, policy requirements and source/output
  identity. Database framing records byte count and SHA-256; independent Rust
  frame verification and canonical vectors remain unfinished.
- Added the serializable association admission procedure with catalog/source
  locks before resource-parent locks, exact active-version readiness, scoped
  relative paths, mode-appropriate durability, immutable source fingerprint,
  target streams and policy snapshots. Runtime grants expose this admission
  entry, not retired profile/absolute-path creation or any helper/table.
  Fingerprint deduplication is keyed by immutable association version and
  relative path, so another configuration version is not treated as an
  unchanged candidate under an older plan. No dependency or approval changed.
- Fresh full-init/sealed-role installation passed. The Linux arm64 real-service
  fixture saved profile/association through authenticated HTTP, then exercised
  admission directly through the restricted runtime role. It produced a
  dry-run job despite a false dry-run input; authenticated job GET returned
  its intent. Five ordered root rows, null backup evidence, identical
  source/output bindings, version 1, positive framing size and 32-byte digest
  were checked through the fixture owner. Unchanged admission returned no
  second job; invalid scope/fingerprint left no extra job; table/helper and
  retired creation calls returned SQLSTATE 42501. The first fixture run failed
  only on its overescaped bytea-display regex; fixing that assertion produced
  the passing result without altering the stored digest or criteria.
- Admission fingerprint values in this procedure test were synthetic, not
  real file-inspection evidence. Valid FFV1 source SHA-256 and saved profile/
  association readback remained unchanged after process/container restart.
  All owned media, volumes, databases, containers and network were removed.
  This is not an HTTP manual-run, worker plan, replacement/recovery or package
  qualification. Existing discovery/recovery workers remain paused/failing on
  retired readers. Retirement/retention callers and old tests still require
  the coordinated cutover; do not count this SQL prerequisite as completion.
- `git diff --check`, matching Linux arm64 compilation and
  `just --command sonar analyze secrets crates/revaer-data/init.sql` passed.
  Full CI/UI, Sonar quality/positive coverage and remote release gates remain
  outstanding. Reviewed root/data instructions and ADR 557's exact admission
  boundary; no criteria relaxation, migration or legacy-equivalence work.
  Rollback removes this unreleased init delta, never operator media.
- Next action: wire the bounded association manual-run request to the new
  procedure with real descriptor-owned fingerprinting; replace worker/recovery
  mutable root readers with claim-fenced immutable snapshots and retained
  root authority, then prove actual inspection and a dry-run plan.

### Retained-Descriptor Fingerprinting Checkpoint, 2026-10-02

- Added read-only candidate-parent traversal beneath a retained catalog root.
  It validates the bounded relative path, rejects symlinks, rechecks root links
  before/after traversal and returns a new open-file description, not the
  lock-bearing root descriptor. Catalog lookup uses trusted source slot order;
  an absent slot is rejected. No files, locks or root authority are manufactured.
- Added the aggregate fingerprint entry point taking an already-open parent.
  Existing path-based callers reuse the same hashing and sidecar-change logic;
  the aggregate format is unchanged. A regression proves a replaced pathname
  does not redirect descriptor-based hashing, and source deletion yields no
  candidate rather than processing the replacement. Bootstrap injection and
  generation-fenced manual-run wiring remain unfinished; this primitive alone
  delivers no new operator workflow or actual plan.
- Four fingerprint regressions and the constrained Linux arm64 parent/catalog
  tests passed. The first Linux launch failed because Cargo changed its working
  directory into the read-only checkout. Running the built test executable in
  the owned `/proof` tmpfs fixed the harness without changing filesystem policy.
  Scoped all-targets strict Clippy passed after borrowing the aggregate context
  rather than suppressing the needless-pass-by-value finding. Formatting and
  diff checks passed. Owned synthetic media and fixture directories were
  removed, including the disposable Linux container/tmpfs.
- Secrets analysis passed for the four edited source/test files; the final
  fingerprint file was rechecked after its lint correction. Exact commands:
  `just --command sonar analyze secrets crates/revaer-media-runtime/src/root_catalog/directory.rs crates/revaer-media-runtime/src/root_catalog/directory/tests.rs crates/revaer-media-runtime/src/root_catalog/opened.rs crates/revaer-app/src/media_discovery_fingerprint.rs`
  and `just --command sonar analyze secrets crates/revaer-app/src/media_discovery_fingerprint.rs`.
  Full quality/coverage and release gates remain outstanding. No dependency,
  approved design, instruction or gate changed; root/Rust instructions and the
  ADR 557 retained-root boundary remain controlling. Rollback removes this
  unreleased traversal entry, never operator media.
- Next action remains the authenticated bounded manual-run cutover: inject the
  retained root/generation collaborator, bind real descriptor-owned fingerprints
  to serializable association admission, then resume claim-fenced worker
  inspection and dry-run planning. Do not add another scope-only preview.

### Authenticated Manual Admission Checkpoint, 2026-10-02

- Operator authorization for the save patch and already-written approved spec
  remains authoritative. No new architecture, dependency or approval hold was
  introduced. Bootstrap now injects its retained source roots and exact numeric
  generation/digest into manual admission; it retains the root lease through
  service shutdown. Admission fingerprints actual media beneath those retained
  descriptors and binds the exact association version, generation and digest to
  the serializable stored procedure. A changed fence rejects admission rather
  than resolving a logical key against another catalog generation.
- Authenticated `POST /v1/media/discovery/runs` now accepts the bounded association
  UUID/root-relative candidate contract, with strict body validation, a 1 MiB
  bound and no-store responses. OpenAPI and the generated test client match it.
  Duplicate, outside-scope, missing and unchanged candidates have explicit skip
  outcomes; the procedure preserves policy-enforced dry-run. Existing automatic
  discovery and worker/recovery cutovers remain unfinished, not waived.
- A constrained real Linux arm64 service with an owned PostgreSQL database,
  ext4 volume and real FFmpeg media admitted exactly one dry-run job through
  authenticated HTTP. Duplicate, outside-scope and missing candidates were
  skipped; repeating the unchanged source created no second job. Authentication,
  unknown-association, retired-body and invalid-relative-path failures passed.
  Exact configuration readback survived process and container restart, and the
  valid source media SHA-256 stayed unchanged. The fixture removed its database,
  containers, network and volume, including all test media.
- This proves real-file admission only: the job stayed queued. Startup recovery
  still fails at the retired mutable profile-root reader in
  `MediaJobRuntime::recover_interrupted_replacements`; no inspection, completed
  plan, execution or recovery is claimed. The older job-read response still
  exposes absolute intent paths and also needs the approved wire cutover.
- Host all-targets strict Clippy passed for API, app and data. The Linux app test
  binary compiled. The real router authentication/body/no-store regression and
  two manual-handler validation regressions passed. OpenAPI export and generated
  API client checks passed; npm audit reported zero vulnerabilities. Linux-native
  Clippy could not run because the pinned compiler image lacks the component;
  the first attempt additionally hit the rustup shim's read-only installation.
  This is an outstanding validation input, not a reason to weaken the gate.
- Secrets analysis ran and passed for the fourteen edited Rust/SQL files using
  `just --command sonar analyze secrets` with their explicit paths. Full Sonar
  quality/positive coverage, `just ci`, `just ui-e2e`, supported shipping package
  qualification and remote PR checks remain outstanding. Root and scoped Rust,
  data and API contracts remain controlling; no criteria were relaxed. Rollback
  removes this unreleased admission delta, never operator media.
- Next action: replace the demonstrated worker/recovery mutable-root dependency
  with approved immutable job evidence and retained root authority, then run
  actual inspection and complete the dry-run plan for the authenticated saved
  configuration. Reuse unchanged save/restart/admission evidence.

### Worker Root Reader Checkpoint, 2026-10-02

- Added the approved ADR 517/557 five-row root reader to init, with runtime-only
  execution, current job/attempt/claim ownership, ready exact catalog generation,
  canonical ordering and persisted byte-count/digest validation. No path/key
  rebinding or direct runtime table access is introduced. Rust independent digest
  verification and worker integration remain unfinished.
- In the owned Linux service/database fixture, runtime-role reads returned five
  rows and rejected wrong attempt and claim generation. This reader proof used
  a synthetic admission/claim in a serializable transaction which was rolled
  back; it is not real inspection or planning evidence. Real-file authenticated
  admission, original SHA-256 preservation and restart readback also passed.
  Owned databases, containers, network, volume and test media were removed.
- Failed harness attempts were a stale embedded init digest, a duplicate Node
  identifier and a race with the live worker; none were waived. The race exposed
  real new evidence: on initial startup before profiles exist, recovery can pass
  and the worker actually claims the admitted job, then fails with
  `media_capability_snapshot_invalid`. After restart with a saved profile,
  recovery still fails through its retired mutable profile-root dependency.
  Both failures remain; the earlier queued status was a momentary observation,
  not evidence that the worker never ran.
- Linux app recompilation, formatting and diff checks passed. Exact incremental
  secrets command `just --command sonar analyze secrets crates/revaer-data/init.sql`
  ran and passed; full quality/coverage and release gates remain outstanding.
  Reviewed the approved ADR 517/557 reader boundary; no approval, dependency,
  instruction or gate changed. Rollback removes this unreleased reader only.
- Next action: diagnose the demonstrated capability snapshot rejection, finish
  the immutable-root worker/recovery cutover and independent Rust verification,
  then prove actual inspection and a completed dry-run plan. No new approval is
  needed and no migration or backward-compatibility investigation is warranted.

### Capability Parser And Live Worker Checkpoint, 2026-10-02

- The actual FFmpeg inventory exposed a parser defect: its six-character legend
  flags were accepted as codec rows with the name `=`. The strict runtime snapshot
  validator correctly rejected that unsupported pseudo-codec. Codec parsing now
  requires exactly six flag characters and excludes the legend delimiter; named
  encoder/decoder/format parsing also excludes that delimiter. No capability
  validation or required utility was removed or weakened.
- A regression uses real-shaped legend and capability rows and retains encode/
  decode flags. All nine capability tests and scoped all-targets strict Clippy
  passed; the Linux real-service test binary rebuilt. Exact incremental Sonar
  secrets command `just --command sonar analyze secrets crates/revaer-media-runtime/src/capabilities/parse.rs crates/revaer-media-runtime/src/capabilities/tests.rs`
  ran and passed. Full Sonar quality/coverage and release gates remain outstanding.
- In the owned real-service fixture the corrected inventory contained no
  unsupported pseudo-codecs, and the real worker passed capability validation,
  claimed the authenticated real-media dry-run job and entered `inspect_plan`.
  It then failed with `media_job_operation_cost_snapshot_invalid`; an entered
  phase is not completed inspection or a plan. The existing seed function creates
  built-in policy parents but supplies no required operation-cost rows. The
  approved spec's weighted defaults at `MEDIA_TRANSCODING.md` lines 1541-1557
  guide the next policy cutover; runtime fallback costs must not hide the gap.
- Process/container restart and exact saved configuration readback passed;
  original media SHA-256 remained unchanged. All owned media/database/container/
  network/volume fixtures were removed. The diagnostic FFmpeg image still is not
  shipping package qualification. No dependency, architectural choice, approval
  or gate changed. Root/Rust instructions remain controlling; rollback removes
  this parser fix, never operator media.
- Next action: supply and verify the approved complete persisted operation costs,
  complete immutable-root worker/recovery integration, then prove actual
  inspection and the completed dry-run plan. Preserve the demonstrated restart
  recovery failure as outstanding rather than reopening resolved approvals.

### Real Dry-Run Plan Checkpoint, 2026-10-02

- Built-in v1 policies now persist all thirteen ADR 518 operation-cost rows.
  Weights follow the spec's defaults; normalized subtitle transcode maps to its
  non-OCR conversion weight, and sidecar removal shares sidecar copy's lightweight
  weight of 2. Seeding only inserts an absent family and never modifies a populated
  or referenced family. The runtime still rejects incomplete/invalid costs.
- Fixed preflight capacity probing to query the configured existing workspace
  root, not its nonexistent projected output directory. No dry-run directory is
  created to satisfy the probe; failures still propagate. Retained-root authority
  and independent snapshot verification integration remain outstanding.
- The constrained real Linux arm64 service now inspects actual FFV1 Matroska
  media and completes a persisted dry-run `video_transcode` plan targeting H.264
  through authenticated HTTP admission. The `inspect_plan` phase completed;
  the planned command uses FFmpeg/libx264 with a persisted argument digest. No
  transcode was executed and the original SHA-256 remained unchanged immediately
  after planning and across process/container restart. Configuration readback and
  admission failure paths passed; all owned fixtures and test media were removed.
- This plan completed before restart. Recovery still pauses the worker after
  restart, so the full UI-save/restart/plan milestone is not yet achieved. Existing
  tests which explicitly seed cost-7 rows into built-in policies need conversion
  to isolated authored test policies, preserving their missing-cost assertions.
  No full CI/UI, shipping package, Sonar quality/coverage or remote gate pass is
  claimed. Linux compilation, scoped all-targets strict app Clippy and incremental
  secrets scanning passed (`just --command sonar analyze secrets crates/revaer-app/src/media_job_runtime.rs crates/revaer-data/init.sql`).
- No dependency or criteria changed. Root/Rust/data instructions and ADR 518's
  complete-cost boundary remain controlling. Rollback removes this unreleased
  seed/probe delta, never operator media. Next action is the approved immutable
  root worker/recovery integration and an actual plan after service restart.

### Worker Claim-Fenced Root Admission, 2026-10-02

- The worker now reads the complete typed five-row immutable root family through
  the approved claim/catalog-fenced procedure before workspace creation. Transport
  requires exactly five ordered kinds from one job; captured source/output and
  workspace paths must match runtime intent. No mutable profile lookup, rebinding,
  default evidence or direct runtime table access is used at this admission.
- The real constrained Linux service still completed the H.264 dry-run plan. A
  second real-service fixture deliberately mismatched the configured workspace;
  the job failed with `media_root_identity_mismatch` before that directory existed.
  Both fixtures preserved media SHA-256 across restart and removed all owned
  media/database/container/network/volume resources. The transport regression,
  scoped all-targets strict Clippy, Linux compilation and secrets scanning passed:
  `just --command sonar analyze secrets crates/revaer-data/src/media/job_roots.rs crates/revaer-data/src/media/mod.rs crates/revaer-app/src/media_job_runtime.rs`.
- Recovery still reads retired mutable roots and processes root-wide journals;
  replacing only its root lookup would not establish per-journal generation
  authority. Next is journal-level recovery fencing and retained descriptor
  integration, plus independent Rust seal verification. The current configured
  workspace equality guard is not the completed per-profile workspace selection
  cutover. No after-restart plan, full gate or production release is claimed.
  No dependencies, approvals or criteria changed; ADR 517/557 and root/data/Rust
  instructions remain controlling. Rollback removes this admission delta only.

### Targeted Journal Recovery Checkpoint, 2026-10-02

- Replacement recovery now exposes an injected one-job journal operation. It
  validates the key and managed-directory ownership, treats absent journals as
  idempotent absence, propagates malformed journal errors and never enumerates or
  recovers siblings. Live commit-failure cleanup uses this operation with the exact
  job/claim key and durable terminal outbox evidence instead of root-wide replay.
- All sixteen host replacement tests passed. Two targeted regressions also passed
  under Linux arm64, UID 501:20, read-only checkout, no capabilities, no-new-privileges
  and disposable tmpfs. They prove rollback/finalization, repeat/missing-journal
  behavior, unchanged sibling media/manifest, and invalid-key/symlink rejection.
  App/runtime Linux compilation and scoped strict Clippy passed. Secrets passed:
  `just --command sonar analyze secrets crates/revaer-media-runtime/src/replacement.rs crates/revaer-app/src/media_job_runtime.rs`.
  Temporary media and containers/tmpfs were removed.
- No new operator workflow is claimed: startup still uses the retired mutable-root
  enumeration. Next is bounded immutable recovery-job enumeration and per-journal
  catalog/attempt authority with retained roots, then actual planning after restart.
  Independent Rust seal verification and full release gates remain outstanding.
  No dependency, approval or criteria changed; ADR 525/557's original job authority
  and root/Rust instructions remain controlling. Rollback removes this targeted
  recovery delta only, never operator media.

### Planning After Restart Checkpoint, 2026-10-02

- Startup no longer reads retired mutable profile roots. It pages claimed,
  non-dry-run attempts (including historical attempts), checks immutable root
  seals and the current catalog generation, and recovers only each exact journal.
  Published terminal outbox records remain usable completion evidence. Runtime
  access is limited to the new security-definer procedure, not table grants.
- In an owned Linux arm64 ext4-volume fixture, the real authenticated service
  saved/read back a profile and association, completed a dry-run video-transcode
  plan, then completed new plans after both process and container restarts.
  Persisted operations were read through HTTP; source hashes stayed unchanged.
  This is real service evidence, not a frontend-only or packaged-release proof.
  The fixture database, roles, containers, network, volume and media were removed.
- Two immutable-root/page transport tests and scoped strict Clippy passed.
  Linux app compilation passed before the final transport-validation-only delta.
  Secrets analysis ran without findings through
  `just --command sonar analyze secrets crates/revaer-data/init.sql crates/revaer-data/src/media/job_roots.rs crates/revaer-runtime/src/media.rs crates/revaer-app/src/media_job_runtime.rs`.
- Still outstanding: retained-descriptor recovery authority, recovery leadership,
  independent Rust seal verification, the connected authenticated UI milestone,
  execution/replacement/recovery evidence and full release gates. Automatic
  discovery still reports a persistence error; its cutover is not complete.
  Next is the connected UI save/restart/plan regression, then execution.
  No architectural choice, dependency or quality criterion changed. Root/data/Rust
  instructions and ADR 512/557 were reviewed; no new policy drift was found.
  Rollback removes this reader/caller delta only, never operator media.

### Manual Discovery UI Checkpoint, 2026-10-02

- Confirmed active manual-association creation now exposes relative-candidate
  preview and explicitly confirmed queue admission. Candidate edits invalidate
  preview/confirmation; failed queue acknowledgement preserves the draft and
  requires renewed confirmation. Complete response accounting rejects foreign,
  omitted or duplicate job rows. No absolute-path or profile-ID write was restored.
- Media UI host tests, host all-targets strict Clippy, WebAssembly compilation,
  bootstrap strict TypeScript checks and instruction-drift checks passed.
  The route-controlled Chromium regression passed at desktop/mobile widths,
  including failure preservation and explicit retry. Screenshots were inspected.
  This does not prove real service persistence, restart or execution through UI.
- E2E Trunk auto-reload is disabled to prevent generated test artifacts from
  erasing browser form state. Compilation/error reporting and coverage gates
  remain enabled. Focused browser invocation still exits nonzero at the mandatory
  aggregate API coverage check; it is not a full `just ui-e2e` pass. Strict wasm
  Clippy failed with widespread UI diagnostics; the reported new semicolon error
  was fixed, without suppressions. Full release gates remain open.
- Secrets analysis ran without findings for the touched media UI files, browser
  test, setup and scoped instructions through `just --command sonar analyze secrets`.
  Temporary fixture roots, database/roles and containers were removed. No new
  dependency, architecture choice or approval was introduced. Root/UI/Rust
  instructions were reviewed and the scoped UI contract updated; no relaxation
  occurred. Rollback removes only this UI/setup delta. Next: connected real-service
  UI save/restart/plan evidence, including selecting persisted associations on reload.

### Connected UI Milestone Checkpoint, 2026-10-02

- Added bounded, validated discovery-association collection reads with runtime
  procedure-only access, complete API/OpenAPI transport, and a paginated UI
  selector. Persisted manual associations are selectable after reload; unavailable
  rows stay visible but disabled. Failed reads and context changes clear stale
  selection. No new dependency or architectural choice was introduced.
- The actual Chromium UI saved an enabled, dry-run-only profile and manual
  association through authenticated HTTP to the real Linux arm64 service. After
  a container restart, the UI reselected the persisted association, previewed and
  explicitly queued the original candidate. The worker completed a persisted
  video-transcode dry-run plan; the original SHA-256 remained unchanged.
  Evidence: `target/connected-ui-proof.log`, job
  `670ecc1d-089b-438d-96b3-70577a9a7dbf`. The host relay forwarded real HTTP,
  including live event streams; no media response was mocked. This used the
  diagnostic Linux test executable/tools, not a shipping package qualification.
  The fixture database, roles, media, volume, network and containers were removed.
- The two new real-service collection API regressions passed after correcting
  errors to `application/problem+json`. The route-controlled browser regression
  also passed reload, retry and failure-clearing behavior. Scoped host strict
  Clippy, model tests, TypeScript checks, formatting, instruction drift, API
  generation and touched-file secrets analysis passed. Focused Playwright runs
  still fail mandatory aggregate coverage; neither is a full `just ui-e2e` pass.
- Root, Rust, data and UI instructions were reviewed; scoped data/UI instructions
  now describe the collection contract. Gates were not relaxed. Rollback removes
  this collection/UI delta only and does not alter operator media. Full CI/UI,
  strict Sonar coverage, package/remote checks and merge remain unproven. Next:
  the same connected operator path with real execution enabled, followed by
  verification, replacement and recovery evidence.
- Execution probe: the UI saved a non-dry-run-only profile, but its selected
  `general` policy retains the separately enforced output dry-run setting.
  Admission therefore remained dry-run and the probe correctly rejected it as
  execution evidence. The current policy editor/API exposes verification and
  video intent, not this output setting; the operator cannot complete that
  configuration path yet. Preview previously showed only the profile restriction;
  the correction below now includes both restrictions. No execution or replacement
  pass is claimed. Next action is the approved output-policy authoring surface,
  with accurate preview, persisted readback and a real execution regression.
  Evidence: `target/connected-execution-proof.log`; owned resources were removed.

### Effective Dry-Run Checkpoint, 2026-10-02

- Association reads now compute effective dry-run from the referenced profile
  and its output policy, failing closed if the output restriction is absent.
  Preview, source admission and destructive-readiness checks use that value;
  the transaction still independently enforces the combined restriction.
- Rebuilt the current Linux arm64 test executable and reran the connected real
  Chromium workflow with an enabled profile whose dry-run-only restriction was
  explicitly cleared, while its selected policy retained dry-run. After restart,
  preview reported dry-run, admission queued dry-run, and the worker completed a
  persisted video-transcode plan. The original SHA-256 stayed unchanged. Evidence:
  `target/policy-restriction-ui-proof.log`, job
  `90c47929-f0f5-474b-8171-e6650736fc03`. This is real service/browser evidence,
  not shipping-package or full-suite qualification. Owned fixture resources and
  media were removed.
- Scoped all-target host strict Clippy and Linux app compilation passed.
  Touched-file secrets analysis, instruction drift and diff checks passed.
  Data/UI instructions now require truthful effective dry-run presentation.
  No new dependency, architecture choice, quality relaxation or migration was
  introduced. Rollback removes this computation/caller delta only.
- Next action remains output-policy authoring through API/UI, then real execution,
  verification and replacement. Full CI/UI, Sonar coverage, package/remote gates,
  review resolution and merge remain outstanding; the goal is not complete.

### Output Policy And Real Execution Checkpoint, 2026-10-02

- The authenticated policy editor/API now authors and reads back all five output
  settings: dry-run, replacement mode, quarantine, permission preservation and
  ownership preservation. Safe defaults remain dry-run with replacement disabled.
  Invalid execution settings preserve the UI draft and leave no policy row.
  Referenced policy versions remain frozen; creation is atomic.
- Policy creation now materializes the existing canonical 13-operation cost table
  in the common factory, not only for built-in versions. This fixes the observed
  `media_job_operation_cost_snapshot_invalid` failure for UI-authored policies;
  runtime missing-cost rejection remains unchanged. No new weights or runtime
  fallback were introduced.
- Real Chromium saved an execution policy, profile and association, restarted the
  real Linux arm64 service, and queued a non-dry-run FFV1-to-H.264 job. The job
  completed with persisted candidate/decode/seek/playback checks, atomic replacement
  and final-graph checks passing; ffprobe confirmed H.264 at the original path.
  Evidence: `target/output-policy-execution-ui-proof.log`, job
  `9a45547f-d8de-4858-b880-16183cd4e0d4`. This diagnostic executable/tools proof is
  not shipping-package qualification. Owned fixture media and resources were
  removed after the run.
- Two focused real-service policy API tests and the real-service policy UI test
  passed, including invalid/duplicate-write rejection and desktop/mobile form
  fit. Their runners still fail mandatory aggregate coverage; neither is a full
  `just ui-e2e` pass. Output model tests, scoped host strict Clippy, TypeScript,
  API generation and Linux compilation passed. These are scoped results, not
  full CI or positive-coverage Sonar qualification.
- Root, Rust, data and UI instructions were reviewed; data/UI instructions now
  describe complete output authoring, exact readback and materialized cost rows.
  No dependency, new architectural choice, migration or gate relaxation was
  introduced. Rollback removes this authoring/factory delta only; it cannot undo
  an operator-approved completed media replacement.
- Next: real failure-path source preservation and recovery evidence, then the
  remaining full-suite, Sonar, package, remote-check and review/merge gates.

### Failure And Retry Checkpoint, 2026-10-02

- Recent jobs now expose status-appropriate retry/cancellation controls. Both
  require explicit confirmation, disable during the request, clear confirmation
  and refresh after success or failure; uncertain writes are never repeated
  automatically. Completed and unknown states expose no mutation control.
- Real Chromium queued a non-dry-run job, then the owned fixture temporarily
  removed source read permissions. The service failed with
  `media_job_runtime_source_fingerprint_failed`; restoring access and comparing
  SHA-256 proved the original bytes unchanged. The UI submitted a confirmed retry
  (HTTP 204), but the retry failed with `media_job_runtime_workspace_failed`.
  Evidence: `target/failure-retry-ui-proof.log`, job
  `45f04dac-717a-4a04-bc2a-3a150cd92ec1`. No successful recovery is claimed.
  Owned media, database and container resources were removed.
- The demonstrated blocker is job-only workspace naming: a retained failed
  workspace collides with creation for the next attempt. Accepted ADR 500 already
  requires attempt/generation-scoped workspaces; implement that with matching
  active retention protection, then repeat real recovery evidence. Do not delete
  previous diagnostics or relax source fingerprint checks to force retry success.
- The pure status-transition test, TypeScript check and touched-file Sonar secrets
  analysis passed. Real Trunk compilation passed during the connected proof.
  The route-controlled browser regression passed confirmation, in-flight disabling,
  rejected retry, accepted retry and cancellation transitions. Its focused runner
  still failed mandatory API coverage collection; this is frontend-only evidence,
  not a full `just ui-e2e` pass (`target/job-actions-ui-test.log`).
  Full CI/UI, strict positive-coverage Sonar and release gates remain outstanding.
  Root/UI instructions were reviewed; UI instructions now require explicit job
  mutation confirmation. No dependency or new architectural choice was introduced.
  Rollback removes these UI controls; it does not alter the existing service API.

### Attempt Workspace And Cancellation Recovery Checkpoint, 2026-10-02

- Managed workspaces now use job/attempt/claim identity through a shared helper.
  Retention snapshots return the same coherent tuple and protect all attempts
  belonging to active jobs, independently active attempts and unpublished terminal
  outcomes. Invalid partial tuples fail closed. Prior diagnostic directories are
  not deleted to make a retry fit; runtime source fingerprint checks are unchanged.
- Queued cancellation previously changed only the job, leaving its attempt queued
  and making retry return HTTP 409. Cancellation now updates both atomically.
  A cancelled-before-claim attempt records null claim/heartbeat timestamps, never
  a fabricated claim. Existing claimed cancellation remains valid.
- Real Chromium saved execution configuration, restarted the real Linux arm64
  service, queued a job and then cancelled it through the service API. SHA-256
  remained unchanged. Confirmed UI retry returned HTTP 204 and completed candidate
  verification, atomic replacement and final-graph checks; ffprobe confirmed
  H.264 at the source path. Evidence: `target/cancellation-retry-ui-proof.log`, job
  `e0248c51-743f-4ae7-916f-c51fcfbb2c74`. This proves this cancellation/retry path,
  not crash-boundary recovery or shipping-package qualification. Owned fixtures
  were removed.
- Scoped workspace tests passed on Linux (nine) and the host (ten, including the
  added cleanup-age protection test). Tests cover nonaliasing, prior diagnostics,
  coherent retention identities, preserved active workspaces and expired inactive
  removal. Touched-source secrets analysis, instruction drift and diff checks
  passed. Full CI/UI, positive-coverage Sonar, packages and remote gates remain open.
- Final scoped all-target host Clippy passed with warnings denied; the ten host
  workspace tests passed again after the test duration was expressed using the
  pinned toolchain's stable `Duration::from_mins` constructor.
- Root, Rust and data instructions were reviewed; data instructions now require
  coherent retention tuples and queued-cancellation attempt state. No new
  dependency, architectural decision, migration or gate relaxation was introduced.
  Rollback removes the paired workspace/retention changes; do not revert only one
  half or run a janitor with mismatched identity rules.
- Next: committed real workflow regression coverage and remaining failure/crash
  recovery evidence, then full quality and release verification.

### Repeatable Operator Regression Checkpoint, 2026-10-02

- Added `just test-media-operator-workflow` with an owned disposable Linux fixture
  and real Chromium driver. It requires explicit immutable tools-image and current
  app-test-executable inputs; it does not silently select a package or skip missing
  prerequisites. The fixture removes its database, containers, network and media
  volume on exit. This is diagnostic regression evidence, not package qualification.
- The versioned driver passed UI output/profile/association persistence, restart,
  cancellation with unchanged source SHA-256, confirmed UI retry, verification and
  H.264 atomic replacement. Evidence: `target/operator-workflow-regression.log`, job
  `c9a8b025-413a-4bc0-9849-9e598d818709`. Existing c8 instrumentation recorded 189
  covered executable lines out of 194 in `coverage/js/media-operator/lcov.info`.
  C8's default test-path filtering was removed for this explicitly instrumented
  driver; Sonar source scope and criteria were not changed. These are local driver
  metrics, not published full-workspace Sonar coverage.
- `just ci` failed at the strict source-inventory guard because existing untracked
  `.playwright-cli` and `output` directories are absent from the configured inventory.
  Neither was excluded or deleted. Evidence: `target/operator-workflow-ci.log`.
- The isolated canonical `just ui-e2e` run failed: 54 passed, 12 failed and 93 did
  not run; required API coverage was missing four entries. The first failures send
  retired path-based profile bodies to the immutable-profile endpoint and receive
  HTTP 400. Evidence: `target/operator-workflow-ui-e2e.log`. An earlier overlapping
  run was invalidated by shared fixed ports and is not product evidence.
- Syntax checks, instruction drift, diff checks and touched-file secrets analysis
  passed. Nondeprecated `sonar analyze --file tests/support/media-operator-workflow.cjs
  --project VannaDii_Revaer --depth DEEP` failed because Vortex is unavailable on the
  connection; its preliminary secrets result is not a structural-quality pass.
- Root, UI and devops instructions were reviewed; devops instructions document
  explicit inputs, cleanup and actual LCOV requirements. Reused Docker, Playwright,
  Node/NVM and c8; no new dependency or architectural choice was introduced.
  Rollback removes this paired recipe/driver only; it never mutates operator media.
- Next: bring API profile regressions onto the approved immutable contract without
  dropping positive persistence or rejection assertions, then rerun full gates.

### Profile Regression Contract Checkpoint, 2026-10-02

- Extended the real-root operator driver with positive profile persistence,
  exact version-1 ETag/Location, unchanged readback after duplicate HTTP 409,
  rejection of physical-path/automation fields without persisted parents, and
  unchanged source SHA-256. These assertions passed before the existing real
  cancellation/UI retry/verified H.264 replacement sequence. Evidence:
  `target/operator-workflow-regression.log`, job
  `d089af66-88f7-4546-9382-1b531fdf632a`. Local driver LCOV reports 221/233 lines
  covered; this is not published full-workspace Sonar coverage.
- No new operator capability was delivered in this checkpoint. Live inspection
  confirmed approved immutable profile `PUT` is not wired and mutable `PATCH`
  remains. The legacy update tests cannot be corrected by merely changing their
  expected status or weakening the persistence assertions. Implement complete
  replacement and version fencing, then align those tests with that behavior.
- An initial driver run timed out waiting for the policy editor before the new
  assertions executed. Added failure title/browser-error/screenshot diagnostics;
  the subsequent run passed. The initial timeout's cause is not established and
  no flake-resolution claim is made. Owned fixture resources/media were removed.
- Driver syntax, touched-file secrets analysis and diff checks passed. Root, UI
  and approved ADR 557/590 contract details were reviewed. No dependency,
  architectural choice, scanner-criteria change or migration was introduced.
  Rollback removes these additional assertions/diagnostics only. Full CI/UI,
  published Sonar coverage, packages and remote review/merge gates remain open.

### Immutable Replacement Checkpoint (2026-10-02)

- Implemented the already-approved ADR 557/590 complete profile PUT. It requires
  one canonical strong latest-version If-Match, appends a version atomically,
  returns the persisted body/new ETag, rejects stale writes with 412 and preserves
  profile keys. PATCH now returns 405 with `Allow: GET, HEAD, PUT`. Updated
  OpenAPI and generated client contracts; no architectural approval is inferred.
- Create and replacement share the private normalized writer and existing root,
  target and policy checks. Only validated public wrappers receive runtime
  execution grants. Existing versions and association references remain immutable.
  Removed now-unused PATCH-only handlers/helpers, not output-policy retention.
- `just test-media-operator-workflow` passed against the complete sealed init,
  restricted runtime role and real Linux service: version 2 persistence, stale
  edit rejection, missing precondition, immutable key and retired PATCH checks,
  authenticated association save, service restart, cancelled-original SHA
  preservation, confirmed retry and verified H.264 replacement. Evidence:
  `target/operator-workflow-regression.log`, job
  `eab09992-bf40-4e72-b20a-d73a3c764238`. This is a diagnostic Linux arm64
  fixture, not shipping-package qualification. Owned media/resources were removed.
- An initial combined run reached replacement successfully but association save
  used the pre-replacement UI profile selection and returned 409. Reloading the
  page before authoring the association selects its persisted current head; the
  subsequent complete regression passed. No stale-write gate was relaxed.
- Six focused API precondition/body/OpenAPI tests, strict scoped all-target and
  production panic-free Clippy, formatting, instruction drift and driver syntax
  passed. Touched-file `sonar analyze secrets` reported zero findings. Targeted
  `sonar analyze --file crates/revaer-api/src/http/handlers/media_preconditions.rs
  --project VannaDii_Revaer --depth DEEP` failed because Vortex is unavailable;
  its preliminary secrets result is not a structural-quality pass or published
  coverage. Full CI/UI, server coverage, package and remote gates remain open.
- Reviewed root, Rust, data, UI, Sonar and approved ADR 557/590 instructions;
  specialized the data instructions for private writer grants and atomic
  replacement. No dependencies, scanner criteria, migration or compatibility
  work was introduced. Existing profile-change events remain the observability
  signal. Risk is an incorrect head/contract transition; rollback removes the
  unshipped replacement implementation, not caller-owned data or safety gates.
- Next outside-in action: align positive profile/discovery regression cases with
  the complete versioned profile and association workflow, then finish operator
  edit/archive controls. Do not change their assertions merely to accept rejection.
- Full `just ui-e2e` was rerun after replacement: 53 passed, 13 failed and 93
  did not run; mandatory aggregate coverage also failed. Retired path-based
  creation/discovery and PATCH expectations still need complete positive-workflow
  replacements; unsupported-method expectations must reflect the approved 405.
- `just ci` was attempted against an owned, sealed disposable init database with
  its restricted runtime login. It stopped in `db-start`, which still invokes
  migrations and received permission denied for schema public. Evidence:
  `target/operator-workflow-ci.log`. No privileges were increased, no migration
  was edited and no caller-owned database was reset. The canonical recipe still
  needs its approved single-init cutover; this is not an architectural hold.
  Both full-gate fixtures were removed. These failed gates prohibit completion.

### Canonical CI Baseline Checkpoint (2026-10-02)

- Addressed the demonstrated CI dependency on migration-backed `db-start`.
  `validate` now calls `db-baseline-verify` against its explicitly initialized
  database. The thin command reuses application startup's read-only packaged
  baseline verifier, exact digest and runtime-login checks and existing ten-second
  connection/statement bounds; it does not create, migrate, reset or repair schema.
  This does not qualify or finish managed development initialization.
- In an owned sealed-init database, the command accepted the constrained runtime
  login and rejected a missing endpoint and privileged admin login. `just ci`
  passed this baseline gate, then stopped in policy validation on the existing
  source-inventory mismatch for untracked `.playwright-cli` and `output` entries.
  Evidence: `target/operator-workflow-ci.log`. No Sonar scope, privilege or
  acceptance criterion was relaxed; caller-owned artifacts remain untouched.
- Added process regressions for missing configuration and redacted invalid-URL
  diagnostics. Existing baseline library tests retain the exact seal/digest,
  malformed-state, runtime-role and denial contract. No operator media capability
  was added by this gate repair, and no full CI pass is claimed.
- Both process regressions, strict all-target data Clippy, formatting,
  instruction drift and diff checks passed. `sonar analyze secrets` passed for
  all touched files. `sonar analyze --file
  crates/revaer-data/src/bin/verify_database_baseline.rs --project
  VannaDii_Revaer --depth DEEP` failed because Vortex is unavailable; no
  structural Sonar pass or published coverage is claimed. The prior full UI
  failure is unchanged; no operator UI/API behavior changed this checkpoint.
- Reviewed root, data, Rust, devops and approved ADR 591 instructions; updated
  data/devops scope for the canonical read-only gate. No dependency was added.
  Observability uses bounded command diagnostics and the existing baseline
  reasons, never raw database errors. Risk is a wrong bootstrap selection;
  rollback removes this unshipped wiring, not seals or caller-owned data.
- Next outside-in deliverable remains complete operator profile editing with its
  current strong version fence, followed by positive workflow-test alignment.
  Full UI, strict published Sonar coverage, package and remote gates remain open.

### Operator Profile Editing Checkpoint (2026-10-02)

- Delivered saved-profile selection, authenticated complete-body loading with its
  exact strong ETag, immutable key display and conditional full-profile saves.
  Success requires the expected next version, identity, complete body and returned
  tag. A stale save preserves the draft and original fence; no automatic retry or
  rebasing is performed. Selecting another profile explicitly confirms discard.
  Unavailable logical roots remain visible in the draft, never silently replaced.
- Pending reads and writes carry an authentication/component scope; obsolete
  responses cannot update the current editor. Authentication changes clear edit
  authority without overwriting an existing draft. This race protection is
  implemented, but no dedicated authentication-race browser qualification is
  claimed by the normal save/stale-save evidence below.
- `just test-media-operator-workflow` passed against the real Linux service and
  sealed init: browser create, complete editor save to version 2, an intervening
  version 3 save, desktop/mobile stale editor saves returning 412 with preserved
  text, association save, restart, cancellation with unchanged original SHA,
  explicit retry and verified H.264 replacement. Evidence:
  `target/operator-workflow-regression.log`, job
  `fee6203c-7d6f-45c2-bfdd-8dbd3d2a16e4`. No package qualification is implied.
- Desktop/mobile captures were visually inspected. The first mobile capture
  caught the responsive sidebar transition; the driver now closes the real
  backdrop and waits for the sidebar to leave the viewport before interaction.
  Real mobile Save clicks receive 412 without losing the draft. Temporary
  screenshots and owned source-media fixtures are removed after inspection.
- Release UI build, all 160 host UI unit tests, strict host all-target and
  production Clippy, formatting and instruction drift passed. The tests include
  exact draft restoration, foreign/missing/weak tag rejection and replacement
  body/head/identity/tag confirmation. Corrected a behavior-preserving discovery
  helper expression required by the existing host lint posture; no suppressions.
- An additional strict wasm Clippy diagnostic failed across the UI, including
  non-Send browser futures and generated macro diagnostics. Structured evidence
  is retained in `target/profile-edit-wasm-clippy.jsonl`; no wasm lint pass is
  claimed and no rule was disabled. A reported redundant clone in this editor
  was removed. Do not mistake a successful release build for a lint pass.
- Touched-file `sonar analyze secrets` passed. Targeted `sonar analyze --file
  crates/revaer-ui/src/features/media/profile_authoring.rs --project
  VannaDii_Revaer --depth DEEP` failed because Vortex is unavailable. Full scanner
  coverage and remote gates remain unqualified; no scanner criteria changed.
- Reviewed root, Rust, UI, Sonar and approved ADR 557/590/591 instructions, and
  specialized the UI instructions for the existing approved editor contract.
  No dependency, architecture decision or migration was introduced. Existing
  profile-change events and explicit save/error UI states provide observability.
  Risk is stale editor authority or false confirmation; rollback removes this
  unshipped editor wiring, not stored versions or safety gates.
- Next action: align the positive profile/discovery suite with the complete
  versioned profile/association workflow and add bounded authentication-race
  regression coverage. Profile archive and remaining release evidence remain
  required; this checkpoint does not reduce the full feature scope.
- Full `just ui-e2e` was rerun: 53 passed, 13 failed and 93 did not run, with
  mandatory aggregate coverage also failing. Evidence:
  `target/profile-edit-full-ui-e2e.log`. Retired path-based profile/discovery and
  mutable PATCH expectations remain the failures; do not change positive tests
  merely to accept rejection. The CI source-inventory blocker from the preceding
  checkpoint remains unchanged. Owned fixtures/media and temporary captures were
  removed; no remote check, push or merge is claimed.

### Resumed Contract Gate Checkpoint

- The operator explicitly authorized the save patch's path-free nullable parent
  columns, atomic profile creation, freezing of referenced policy components,
  and runtime procedure grants under ADRs 557/590. Continue single-agent under
  the already-approved specification; this is not authority for new design
  choices or quality-gate relaxation.
- The API route regression now exercises complete PUT and proves retired PATCH
  returns 405 with `Allow: GET, HEAD, PUT`. Both targeted real-service assertions
  passed; the targeted command still failed mandatory aggregate coverage.
  Evidence: `target/media-route-contract.log`. No new operator capability was
  delivered by this test correction.
- The Sonar inventory guard now follows the root instruction's tracked
  top-level inventory. Its regression accepts an untracked diagnostic directory
  and rejects that same new source once tracked without a corresponding
  `sonar.sources` entry. `just workflow-guardrails-test` passed. No scanner
  property, source filter, quality threshold or server setting changed.
- `just ci` passed that former blocker and reached the transition-only database
  candidate hash/count check, which rejected the changed init body against its
  frozen candidate. Evidence: `target/operator-workflow-ci.log`. The frozen pin
  was not changed merely to pass; CI remains failed.
- Full `just ui-e2e` finished with 54 passed, 12 failed and 93 not run, plus
  aggregate coverage failure. Evidence: `target/resumed-full-ui-e2e.log`.
  Next: move the remaining positive profile/discovery assertions onto real
  catalog-backed profile versions and associations, preserving successful save,
  readback, admission and unchanged-source assertions. Do not replace positive
  workflow tests with rejection expectations or qualify Linux root readiness
  using a host-only fixture.
- Touched-file Sonar secrets analysis passed. Structural analysis remains
  unavailable on the configured connection; neither a structural pass nor
  positive published coverage is claimed. Owned test databases, temporary roots
  and media were removed by fixture teardown. User artifacts and the active
  integration worktree remain intact; no push, merge or remote pass is claimed.
- Stale-policy check: reviewed root policy and the UI/DevOps scoped instructions;
  corrected only the tracked-inventory implementation mismatch. No dependency
  or observability change. Rollback is the narrow test/guard change, not a reset
  of user data or integration edits.

### Restarted Dry-Run Operator Milestone

- `just test-media-operator-workflow` passed with a second complete profile and
  association saved through the authenticated browser. The dry-run profile uses
  the execution-capable output policy while retaining its default profile
  dry-run restriction. After a real service restart, exact profile readback and
  ready association selection passed; the operator previewed and queued the
  relative candidate through the UI.
- Job `5b3f4df5-dc5b-44b7-b12c-584120475e9c` completed as dry-run with persisted
  video-transcode plan operations and an unchanged source SHA-256. The separate
  execution fixture still proved cancellation preserves original bytes and
  confirmed UI retry completes H.264 replacement, job
  `593e00c7-4215-4d9c-9f63-79a16c15438a`. Evidence:
  `target/operator-workflow-regression.log`.
- Fixtures use distinct `DryRun` and `Movies` association prefixes and separate
  source files. An earlier whole-root overlap correctly returned 409; the
  fixture was corrected without changing the service's overlap criterion.
- This is real service/worker evidence, not frontend-only route mocking. It is
  still the explicitly configured Linux diagnostic tools fixture, not shipping
  amd64/arm64 package qualification or complete crash-recovery qualification.
  Local driver LCOV was regenerated; published workspace coverage is unproven.
- Syntax, touched-file secrets and diff checks passed. The preceding full CI
  and UI gate failures remain unresolved, with their relevant inputs unchanged;
  this recipe is not a substitute for either gate. Next: connect the remaining
  canonical positive media API tests to a qualified Linux catalog fixture,
  retaining create/readback, conditional replacement, discovery and diagnostic
  assertions. Do not rewrite them as expected rejections.
- Owned databases, containers, volumes, media and temporary captures were
  removed. No dependency, production behavior, quality criterion, push or merge
  changed. Reviewed the root and UI instruction files; the UI specialization
  now explicitly requires this restart/dry-run proof before execution evidence.

### Shared Native API Fixture Checkpoint

- Extracted the existing disposable Linux service lifecycle into
  `scripts/with-media-test-service.sh`. The operator recipe remains its consumer;
  no database, root-attestation, authentication or teardown criterion changed.
  The API consumer uses the same streaming relay on an ephemeral loopback port,
  owns a separate restricted service/database/volume per worker and never
  overwrites the host suite's authentication session. Readiness credentials stay
  in the private worker pipe and are not logged.
- The positive native profile regression passed under both `api-none` and
  `api-api-key`: logical catalog-backed creation, exact readback/strong ETag,
  complete conditional replacement, stale 412 and unchanged saved head.
  Evidence: `target/media-api-fixture.log`. The targeted command still failed
  mandatory aggregate coverage, which was not bypassed. No obsolete positive
  test was deleted or replaced with rejection expectations.
- Full `just ui-e2e` reached that new native test and finished with 55 passed,
  12 failed and 94 not run, plus aggregate coverage failure. The same retired
  profile/discovery contracts remain the failures. Evidence:
  `target/media-fixture-full-ui-e2e.log`. Next: migrate those assertions to
  `tests/fixtures/media.ts`, with owned per-test source prefixes and real media,
  then qualify association-based admission, diagnostics and disabled triggers.
- The extracted-wrapper operator regression passed again, including restart,
  unchanged-source dry-run planning, cancellation and confirmed execution retry.
  Driver and extracted relay both remain instrumented in its regenerated local
  LCOV. The diagnostic image/current Linux app executable must be supplied
  explicitly; this is not shipping package proof or published workspace coverage.
- Strict harness compilation, shell syntax, touched-file secrets and diff checks
  passed. The preceding CI frozen-candidate failure and unavailable structural
  Sonar connection remain unresolved, with their inputs unchanged. No new
  dependency or production behavior; no push, merge or complete gate pass.
- Reviewed root, UI and DevOps instructions; scoped notes now describe shared
  fixture ownership, private credentials and the distinction between readiness
  and workflow evidence. Owned service resources/media and temporary captures
  were removed. Rollback is the fixture extraction/test change, not caller data
  or preserved integration edits.

### Native Profile Contract Migration Checkpoint

- Migrated profile-create/update assertions to the owned Linux API fixture.
  Each requested fixture owns a UUID-named source prefix, copies real FFV1 media,
  seeds explicit target/policy versions and removes its exact directory in
  `finally`, including failed setup/assertions. It no longer invents host roots
  or uses fake media bytes. Save, duplicate conflict, complete fenced PUT and
  invalid automation-field writes retain exact readback and byte-preservation
  assertions; automation fields belong to associations, not profile bodies.
- Targeted source-level tests passed 16 and failed six across both authentication
  modes. The failures now demonstrate real service gaps, not merely obsolete
  test bodies: native profile readiness and disabled schedule/watcher requests
  return 500. Their current implementations still load the retired parent
  descriptor. Evidence: `target/profile-contract-migration.log`.
- The manual test now creates an explicit, prefix-scoped association and submits
  relative candidates. Its readiness assertion fails before admission, so
  downstream planning, duplicate admission and diagnostic assertions have not
  yet passed in that test. They remain present; do not report them as qualified
  or replace the expected successes with failures.
- Full `just ui-e2e` finished with 63 passed, four failed and 94 not run, plus
  aggregate coverage failure. Evidence: `target/profile-migration-full-ui-e2e.log`.
  Three failures are the native endpoint 500s above; the broad media workflow
  still submits retired creation fields. The prior claim that failures were
  solely stale tests was incomplete: valid native requests expose service gaps.
- Next: repair readiness under ADR 557 using latest/active immutable versions,
  per-kind root readiness, active association count and bounded path-free
  reasons. Then complete association-based automation/planning rather than
  fabricating legacy paths, filling nullable columns or loosening readiness.
  Existing operator approval covers these workflow requirements; no new hold
  or architectural approval request is introduced by an implementation failure.
- Strict harness compilation, touched-file secrets and diff checks passed.
  No production behavior, dependency or quality gate changed in this checkpoint.
  The earlier frozen-candidate CI failure and unavailable structural Sonar
  connection remain unresolved. Reviewed root and UI instructions; their
  requirement to preserve positive workflow assertions was followed. Owned
  test resources/media were removed; no push, merge or completion claim.

### Native Profile Readiness Checkpoint

- Implemented path-free profile readiness from latest/active immutable heads,
  per-kind root readiness and active-version association count in one read-only
  repeatable-read snapshot. Runtime receives procedure grants, not table access.
  Reporting does not enable the profile or replace admission/execution checks.
  The existing ADR 557/590 authority and explicit save-patch approval apply;
  this implementation introduces no new architectural hold or dependency.
- Updated OpenAPI and the generated client. Planning now uses the approved
  association identity and relative candidate. The real Linux service passed
  readiness, planning, manual admission, duplicate handling, diagnostic reads,
  unchanged profile readback and exact source-byte preservation in both
  authentication modes. After creating an association, readiness reports one
  active association. This is real service evidence, not frontend-only proof.
- Readiness unit tests passed, including distinct draft/latest and active
  bodies, absent active heads, mismatched identities and malformed counts.
  Strict all-target Clippy passed after removing a redundant clone in an
  association-response test, without changing its assertions. Generated client
  compilation, npm audit, touched-file secrets scanning and diff checks passed.
- Full `just ui-e2e` failed: 64 passed, three failed and 94 not run, with
  aggregate coverage failure. Remaining failures are schedule/watcher 500s
  from the retired profile-level path and the broad media test's retired
  profile request. Evidence: `target/native-readiness-full-ui.log`;
  targeted both-authentication evidence: `target/native-readiness-api.log`.
- `just ci` verified the current packaged baseline and restricted runtime role,
  then failed on historical candidate hash/count equality in
  `scripts/tests/database-final-test.rb`. The approved feature-development
  boundary still needs to be applied to this ordinary-CI prerequisite; no
  frozen pins or checks were changed. Evidence: `target/operator-workflow-ci.log`.
- Structural Sonar remains unavailable on the previously verified connection;
  secrets analysis is not structural analysis or positive published coverage.
  No full release, package qualification, remote-check pass, push or merge is
  claimed. Owned disposable databases, containers, volumes and test media were
  removed by fixture teardown. The active integration worktree is preserved.
- Next: implement association-based schedule/watcher requests and migrate the
  broad media workflow while retaining positive admission and safety assertions.
  Align the stale CI prerequisite with approved ADR 591 without dropping fresh
  init, sealed-runtime, privilege, transaction or workflow qualification.
  Reviewed root, Rust, data, UI and DevOps instructions; scoped data/UI guidance
  now describes the native snapshot reader. No criteria relaxation was made.

### Native Automation Request Checkpoint

- Schedule/watcher requests now share manual discovery's exact bounded,
  association-relative request contract. Retired profile-ID and absolute-path
  bodies are rejected before reading state. Existing native associations report
  their disabled-mode error without loading nullable parent paths or admitting
  jobs. Unknown associations return 404. Automatic activation remains
  unqualified, not removed or newly awaiting architectural approval.
- Real-service assertions passed for both authentication modes: disabled modes,
  unknown association, empty/129-candidate lists, absolute paths and retired
  profile-ID bodies leave jobs empty and source bytes unchanged. All 22 targeted
  profile/discovery assertions passed; the command still failed mandatory
  aggregate coverage because it ran only this file. Evidence:
  `target/native-automation-api.log`. Four handler-boundary unit tests passed.
- Moved the broad media test from fabricated host media to the owned Linux
  fixture. Complete fenced PUT now proves metadata version 2 and selected-target
  version 3 with persisted readback and unchanged source bytes. The old target
  PATCH had reported success without changing the native head; it remains a
  service-surface issue to retire under the approved full-replacement contract.
  Positive automation, export/import and downstream job assertions remain;
  reaching an earlier assertion does not qualify those later workflows.
- Latest full `just ui-e2e`: 65 passed, two failed, 94 not run, plus aggregate
  coverage failure. The observed failures are schedule listing and YAML export
  returning 500 on native profiles. Evidence: `target/native-automation-full-ui.log`.
  The next service changes must use native version/association readers, not
  restore physical paths or fill retired nullable columns.
- Both strict all-target and production Clippy passed for changed API/model
  crates. OpenAPI/client regeneration, npm audit, touched-file
  `sonar analyze secrets` and diff checks passed. Structural Sonar remains
  unavailable on the previously verified connection; no published coverage,
  package qualification or remote-check result is claimed.
- `just ci` verified the current sealed baseline/restricted runtime and again
  failed historical candidate equality in `database-final-test.rb`. Apply the
  already-approved ADR 591 feature-development boundary without weakening fresh
  init, privilege, transaction or workflow qualification. No frozen hash or
  criterion changed. Evidence: `target/operator-workflow-ci.log`.
- Reviewed root, Rust, data, UI and DevOps guidance; updated scoped UI instructions
  for native automation request validation. No new dependency or architectural
  decision. Owned databases, containers, volumes and media were removed by
  fixture teardown; the incomplete integration worktree is preserved. No push,
  merge or completion claim. Rollback is the scoped request/test change, not
  caller data or other preserved integration edits.

### Native Automation Listing Checkpoint

- Schedule/watcher listing now uses the canonical validated association page
  reader, not nullable path-taking parent reads. Existing list envelopes contain
  complete path-free association bodies, requested discovery modes and the
  canonical continuation. Bounds, ordering and cursor membership are owned by
  the shared reader; reporting modes is not automatic-activation authority.
  No schema, grant, dependency or architectural hold was added.
- Updated OpenAPI and regenerated the client. Real Linux service tests passed
  exact association readback in both lists, bounded pagination/continuation,
  zero/201 limit rejection, malformed cursor and unknown-query rejection, and
  absence of physical paths. Source bytes remained unchanged and disabled
  requests admitted no jobs. All 22 targeted assertions passed in both auth
  modes; the targeted command still failed mandatory aggregate coverage.
  Evidence: `target/native-automation-list-api.log`.
- Two default-facade unit tests passed; both strict all-target and production
  Clippy passed for changed API/model crates. Touched-file
  `sonar analyze secrets`, npm audit, generated client and diff checks passed.
  Structural Sonar remains unavailable on the previously verified connection;
  secrets scanning does not prove structural quality or published coverage.
- Full `just ui-e2e`: 65 passed, two failed and 94 not run, plus aggregate
  coverage failure. Both observed failures now reach `GET /v1/media/export`,
  which returns 500 from the retired profile reader. Listing no longer fails.
  Evidence: `target/native-automation-list-full-ui.log`. The YAML compiler and
  import/export model still use retired path fields; do not fix export by
  inventing paths, hiding native profiles or claiming the later import and
  automation assertions passed.
- `just ci` again verified the packaged baseline/restricted runtime, then failed
  unchanged historical candidate hash/count equality. Evidence:
  `target/operator-workflow-ci.log`. The approved ADR 591 CI alignment remains
  outstanding; no frozen pins, security requirements or release gates changed.
- Next: implement ADR 557's logical-root profile and association YAML contract
  through real export, validation and atomic apply, retaining canonical limits,
  forced import dry-run and transactional failure assertions. Then align the
  stale ordinary-CI prerequisite with the already-approved v0 boundary. Existing
  target-PATCH false-success and automatic qualification remain outstanding.
- Reviewed root, Rust, data, UI and DevOps instructions; scoped UI notes now
  describe native paged listings. Owned test databases, service containers,
  volumes and media were removed; the active integration tree and other user
  resources were preserved. No push, merge or completion claim. Rollback is the
  scoped listing/model/test change, not caller data or unrelated integration work.

### Retired Target-Only Write Checkpoint

- Removed the target-only profile PATCH route, handler, obsolete DTO and
  OpenAPI/client operation. The approved complete conditional profile PUT is
  the write authority; the retired writer could acknowledge a target change
  without updating the native immutable profile head. No architecture or
  dependency changed, and no acceptance criterion was relaxed.
- Real Linux service tests under both authentication modes rejected target pin
  and clear requests with 404, preserving the complete native profile, ETag and
  source bytes. A subsequent complete fenced PUT succeeded at version 2.
  All 24 targeted tests passed; the command exited unsuccessfully because the
  mandatory aggregate coverage check requires the remaining suite. Evidence:
  `target/retired-target-patch-api.log`. These are real API/service tests, not
  frontend mocks or package qualification.
- OpenAPI route and desired-target graph unit tests passed. API/models strict
  all-target/all-feature and panic-free production Clippy passed, as did format,
  instruction drift, client generation and npm audit (zero vulnerabilities).
  `sonar analyze secrets` passed for the changed sources, instructions and
  generated artifacts; this is not structural analysis or published coverage.
  Logs use the `target/retired-target-patch-` prefix.
- `just ci` verified the sealed baseline and runtime identity, then failed the
  unchanged historical candidate equality prerequisite. `just ui-e2e` had 66
  passes, two export-500 failures and 95 not run, with aggregate coverage also
  failing. Evidence: `target/retired-target-patch-ci.log` and
  `target/retired-target-patch-full-ui.log`. No full-gate, push or merge claim.
- Reviewed root, Rust and UI instructions; UI guidance now prohibits the retired
  target writer and requires preservation evidence. Rollback is limited to this
  route/schema/test change, not unrelated integration work or caller data.
- Next: native logical-root export, validation and atomic import with explicit
  versions, canonical bounds, disabled unmapped drafts and forced import dry-run.
  Historical ordinary-CI alignment, automatic qualification and remaining
  release evidence stay outstanding. No new operator approval is requested.

### Native Portable Export Checkpoint

- Portable `GET /v1/media/export` now reads native immutable profile heads and
  every latest association's exact profile pin, complete logical bindings and
  catalogs in one restricted read-only repeatable-read transaction. New bounded
  stored procedures are explicitly granted to the runtime; no raw runtime table
  access, migration, path placeholder, dependency or architectural change was
  introduced. Older association-pinned bodies remain in the export after a
  profile advances. Serialized associations contain only the eight ADR 557
  logical fields, including an explicit whole-root empty prefix.
- Canonical resource ordering, positive exact profile versions, complete native
  request validation and reference checks precede serialization. Retained
  overflow lookahead rejects more than 128 resource rows, 4,096 logical-binding
  and stream children or 4 MiB of output rather than returning a truncated
  bundle. A malformed or missing referenced body fails closed. Profile intent
  is preserved on export; forced import dry-run remains an import obligation.
- The real Linux service passed native export tests under both authentication
  modes: version-1 and version-2 bodies matched exactly, the association retained
  its version-1 pin, repeated export was identical, no host path/UUID/attestation
  fields were emitted and source bytes remained unchanged. Evidence:
  `target/native-portable-export-api-focused.log`. Both focused tests passed;
  mandatory aggregate coverage still made this subset command exit unsuccessfully.
  The earlier attempt failed on a test URL typo before reaching export; that
  fixture error was corrected without altering service behavior.
- Five focused unit tests passed for exact bodies/versions, explicit whole-root
  intent, dangling-pin rejection, overflow rejection and deterministic output.
  Data/app strict all-target/all-feature and panic-free production Clippy,
  formatting, instruction drift, diff checks and `sonar analyze secrets` passed.
  Logs use `target/native-portable-export-`. Secrets scanning does not establish
  structural Sonar quality, positive published coverage or package qualification.
- `just ci` verified fresh sealed init/runtime identity before failing the stale
  historical candidate equality prerequisite. Full `just ui-e2e` passed export
  and reached native import validation, which returned 400 from the still-retired
  YAML parser: 68 passed, one failed, 96 not run and aggregate coverage failed.
  Evidence: `target/native-portable-export-ci.log` and
  `target/native-portable-export-full-ui.log`. Neither full gate passed.
- Reviewed root, Rust, data and UI instructions; scoped data/UI guidance now
  distinguishes bounded native export from atomic import qualification. Existing
  local-path export, pure YAML validation, explicit expected-version/create-intent
  application, SERIALIZABLE source-before-parent lock ordering and rollback
  evidence remain incomplete. No round-trip, push, merge or release claim.
- Next: make the YAML validator and atomic applier consume this native logical
  contract, preserving ADR 521 parser-abuse limits, resource fences and ADR 557
  disabled unmapped drafts. Rollback is limited to the native export reader,
  serializer and tests; preserve unrelated integration changes and caller data.

### Native YAML Parser Dependency Rationale

- ADR 521 requires rejecting aliases and recursive YAML before materialization.
  The existing Serde YAML interface resolves aliases during deserialization and
  does not expose parser events. Add `yaml-rust2` with default features disabled
  for its safe Rust event API, solely as bounded syntax preflight. Keep existing
  Serde decoding and native JSON request constructors as semantic authority.
  This implements the approved parser-abuse contract, not a new resource design.
- Do not use textual searches for `*`, `&` or `!`: those are legitimate quoted
  or block-scalar description contents. Parser events distinguish syntax from
  content. Reject multiple documents, aliases, anchored recursive structures,
  custom tags, excessive nesting, duplicate and non-string mapping keys before
  compiling any resource; validate the dependency graph under existing gates.
- Dependency removal rolls back only this syntax preflight. No architecture,
  import authority, data format version or release criterion is relaxed.
- The selected 0.11 release line reuses the existing `hashlink`/`hashbrown`
  versions; 0.12/0.13 would add duplicate versions prohibited by the dependency
  gate. No duplicate tolerance was added. The lockfile selects 0.11.1, with
  default encoding features disabled; only the parser and `arraydeque` are new.

### Native YAML Validation Checkpoint

- Native YAML validation now decodes complete immutable profile bodies through
  the shared JSON request constructor, checks explicit versions, exact catalog
  references and association pins, and produces blocking pointer-addressed
  issues without writes. Retired path-taking profiles are not accepted by this
  compiler. No migration, architectural change or approval reinterpretation.
- Event preflight rejects aliases, recursive input, custom tags, multiple
  documents and nesting beyond 64 before materialization. Serde rejects
  duplicate keys; the compiler rejects non-string keys, unknown fields,
  resource/child overflow and documents above 4 MiB. Quoted and block-scalar
  punctuation remains ordinary text. Import handlers bound the decoded document
  separately from its JSON escaping envelope and return a bounded 413 error.
- Real Linux service tests passed under both authentication modes: native export
  validates, missing exact catalog references produce blocking issues, invalid
  apply attempts cannot mutate profiles, the exact 4 MiB boundary validates,
  oversize input receives 413, malformed documents receive 400, and source bytes
  remain unchanged. Four focused tests passed; subset aggregate coverage failed
  as expected. Evidence: `target/native-yaml-compiler-api.log`.
- The first boundary run exposed the test relay forwarding curl's interim
  `100 Continue` as a final response. The relay now consumes informational
  headers; the unchanged boundary assertions passed on rerun. This is a test
  transport repair, not a production gate relaxation.
- Eleven export/compiler unit tests, strict API/app all-target/all-feature and
  panic-free production Clippy, formatting, instruction drift, dependency audit,
  dependency policy, OpenAPI/client generation and secrets scans passed. Evidence
  uses `target/native-yaml-compiler-` and `target/native-yaml-relay-secrets.log`.
  Secrets scanning and local LCOV are not published structural Sonar coverage.
- Full `just ui-e2e` reached positive native import and failed its unchanged 201
  assertion with 400: 69 passed, one failed, 97 not run; aggregate coverage also
  failed. `just ci` passed fresh-init/restricted-runtime verification then failed
  the historical candidate byte/count prerequisite (1730 versus 1624 statements).
  Evidence: `target/native-yaml-compiler-full-ui.log` and
  `target/native-yaml-compiler-ci.log`. Neither completion gate passed.
- Next: replace the old apply parser/writer with approved native atomic import,
  retaining explicit resource fences, lock order, rollback and forced dry-run.
  External association-only pins, complete catalog lookup, local-path export,
  round-trip, automatic execution, release packages and remote gates remain
  unqualified. No push, merge or release claim. Review scope: root, Rust and UI
  instructions; UI guidance now records native validation and relay boundaries.
  Rollback only the native compiler/transport delta; preserve unrelated changes.
  Owned test containers, filesystem fixtures and media are removed after runs.

### Native Profile Write Lock Checkpoint

- While preparing native atomic import, corrected the shared profile writer's
  lock order under ADR 557: catalog generation fence first, applicable in-place
  source-attestation advisory lock second, target/policy/profile parents last.
  Existing typed create/replace validation, immutable heads and version fences
  remain unchanged. This implements existing approval, not a design change.
- An owned disposable-database fixture holds the source advisory lock while the
  real authenticated service attempts a full profile replacement. It observes
  the waiting backend and proves no target, policy or profile relation lock is
  held, releases the source lock, then verifies successful version-2 persistence
  and unchanged source bytes. Both authentication modes passed. Six focused
  create/lock/export tests passed; whole-suite aggregate coverage makes that
  subset command unsuccessful. Evidence: `target/native-profile-lock-api.log`.
- Linux arm64 build, TypeScript, formatting, instruction drift, diff checks,
  OpenAPI generation and changed-file secrets scans passed. `just ci` verified
  fresh sealed init and restricted runtime before the unchanged historical
  candidate byte/count failure. Evidence uses `target/native-profile-lock-`.
- This is one writer prerequisite, not an operator-visible import deliverable
  or proof of multi-source ordering, atomic bundle rollback, explicit import
  fences, unmapped drafts, structural Sonar coverage or release qualification.
  Next remains complete native atomic import; no new architectural hold. Review
  scope: root, Rust, data and UI instructions. Data guidance now pins the live
  single-source proof and its limits. No new dependency or runtime grant.
- Rollback only this lock-order correction and its owned test fixture; retain
  unrelated integration changes. Test teardown removes owned databases, roles,
  containers, volumes and media. Full `just ui-e2e` reached the unchanged positive
  import assertion and received 400 rather than 201: 70 passed, one failed,
  98 not run, aggregate coverage failed. Evidence:
  `target/native-profile-lock-full-ui.log`. Both mandatory full gates remain
  failing; no new operator-visible capability, push, merge or release is claimed.

### Native Import Intent Checkpoint

- Implemented ADR 521's explicit resource-authority requirement in the request
  envelope, separate from portable YAML: `preconditions` contains one entry per
  distinct native kind/key, with `intent: create` or `intent: match` plus a
  positive `expected_version`. Kind names match native bundle arrays. Multiple
  exported versions of one profile require one head fence, not inferred consent
  from each body version. Validation remains read-only without this envelope.
- Reject absent/incomplete authority with 428, malformed/null/duplicate/extra
  intent with 400, and more than 128 entries with 413. Preserve the 4 MiB decoded
  document limit; only the bounded JSON envelope reserves space for these
  explicitly bounded entries and escaping. No source path, imported identity,
  default version or implicit create/overwrite authority was introduced.
- Six focused real-service tests passed under both authentication modes:
  missing/invalid/overflow authority is rejected, malformed documents and byte
  boundaries remain enforced, native export validates, and persisted profile
  bodies/source bytes remain unchanged. Subset aggregate coverage fails rather
  than being suppressed. Evidence: `target/native-import-intent-api.log`.
  Thirteen compiler/export unit tests passed, including independent body/head
  versions, exact resource coverage and parser-abuse cases.
- The data adapter now opens the existing apply transaction as serializable;
  application error paths explicitly roll it back. Successful native writes,
  late-error rollback, locked head/create comparison, multi-source lock order,
  unmapped disabled drafts and the bounded import audit are still unqualified.
  The old apply parser/writer remains to be replaced. This checkpoint is not
  atomic import completion. UI import confirmation is unfinished; its caller
  does not manufacture authority from YAML and therefore cannot apply yet.
- Strict data/API/app/UI Clippy, TypeScript, Linux arm64 build, generation,
  formatting, instruction drift and secrets checks passed. Evidence uses
  `target/native-import-intent-` logs. No new dependency or grant. Secrets scans
  do not establish full structural Sonar or positive published coverage. Full
  gates remain required; CI stops at historical candidate equality.
- Full `just ui-e2e` reaches native apply with explicit fixture-owned head
  preconditions but still receives 400 rather than its required 201: 71 passed,
  one failed, 99 not run; aggregate coverage failed. Evidence:
  `target/native-import-intent-full-ui.log`. `just ci` passed sealed-init/runtime
  verification before the unchanged historical equality prerequisite failure.
  Neither full gate passes. Owned test media and resources are cleaned.
- Reviewed root, Rust, data and UI instructions; scoped data/UI guidance now
  distinguishes request authority and serializable setup from database-fence
  and complete workflow qualification. Next: consume native compiled resources
  in one locked, audited transaction and prove successful/failed round trips.
  Rollback only this envelope/compiler/transaction delta, not other integration
  changes. No push, merge, release or additional architectural approval claim.

### Native Import Database Fence Checkpoint

- The operator explicitly authorized path-free nullable parent columns, atomic
  profile creation, frozen referenced policy components and runtime procedure
  grants under ADRs 557/590, reaffirming previously authorized specification work.
  This is implementation authority, not a quality-gate waiver or a new hold.
- Added a runtime-granted preparation procedure that holds the root catalog,
  ascending applicable source advisory locks, then resource parents in fixed
  kind/key order. Explicit create intent rejects existing keys; match intent
  compares the locked latest head. Catalog factories share the catalog lock so
  they cannot advance a resource head around import preparation. The app invokes
  this procedure inside its serializable transaction before any legacy writer.
- This is preparation only: native resource writes, unmapped disabled drafts,
  bounded audit and UI confirmation remain incomplete. The positive native
  import assertion still requires 201; preparation does not satisfy it.
- Updated the existing build-only `cc` dependency from 1.2.48 to 1.4.7 (and its
  resolved tool helpers), without adding a direct dependency. An owned host
  build stalled in compiler-family detection: a sampled clang process was
  blocked in `write`, while the pinned helper read stdout and stderr sequentially.
  The updated helper uses `wait_with_output` to drain both pipes. Retain compiler
  diagnostics and rerun verification; no warning or gate suppression is allowed.
- Review scope remains root, Rust, data and UI instructions. Scoped data guidance
  records the preparation order and its qualification limits. Rollback only this
  preparation/build-helper delta; preserve unrelated integration changes.
- Four focused real-service tests passed in both authentication modes: stale
  heads, create-existing and match-missing requests return 409 without writes;
  the source-before-parent replacement regression also passes. Subset aggregate
  coverage fails rather than being suppressed. Host/Linux arm64 builds, strict
  all-target/all-feature and production Clippy, TypeScript, formatting,
  instruction drift, dependency audit/deny and changed-file secrets scans pass.
- Fresh sealed init, restricted runtime identity and denied-input verification
  pass. `just ci` still stops at historical candidate equality (1732 statements
  versus frozen 1624); do not repin historical evidence to manufacture a pass.
  Full `just ui-e2e` still fails at native apply (400 versus required 201):
  72 passed, one failed, 100 not run, aggregate coverage failed. Neither mandatory
  full gate passes. Evidence: `target/native-import-fence-` logs.
  No structural Sonar, published coverage, package, push or merge is claimed.

### Native Import Catalog Correction

- Catalog reuse now requires exact key/version and equal complete represented
  body. Changed immutable bodies conflict instead of being skipped or upserted;
  identical bodies under new keys are actually created. Apply reports a purely
  immutable-catalog conflict as 409, while any malformed-config issue retains
  400. Read-only validation still exposes its blocking conflict issues.
- Two real-service tests passed (both authentication modes), covering target,
  policy and compatibility families, unchanged exports after rejected bodies,
  persisted identical new-key copies and unchanged source bytes. Subset
  aggregate coverage remains failing. Linux arm64 build, TypeScript, production
  Clippy, formatting, instruction drift and changed-file secrets scans pass.
  Evidence: `target/native-import-catalog-` logs. No dependency or architecture
  change. Root/Rust/data/UI instructions were reviewed; no policy drift changed.
- Native profile/association writes, unmapped drafts and the bounded audit are
  still unfinished. This catalog correction is not an atomic bundle milestone.
  The full-suite run exposed a retired path-profile read in catalog-only apply:
  it returned 500 when native profiles existed. Removed that unnecessary read;
  the strengthened real-service test first creates a native profile and then
  passes under both authentication modes. No database error is suppressed.
  Final `just ui-e2e`: 73 passed, one failed, 101 not run, aggregate coverage
  failed; native profile apply still returns 400 rather than required 201.
  `just ci` passed sealed-init/runtime and denied-input verification before the
  unchanged 1732-versus-1624 historical fingerprint failure. Strict all-target
  Clippy passes. Both mandatory full gates remain failing; cleanup removes
  owned disposable databases, containers, test media and captures.
  Next remains native immutable writes in the prepared serializable transaction,
  then authenticated save/restart/dry-run proof. No push or merge is claimed.
  Rollback only this catalog correction; preserve other integration changes.

### Native Bundle Writer Checkpoint

- Replaced runtime path-profile import with native body-version and association
  writers inside the prepared serializable transaction. Imported profiles force
  dry-run, exact existing bodies are immutable comparisons, and unresolved roots
  persist nullable-attestation disabled drafts without replacing active heads.
  Associations retain exact profile-version pins; watcher/schedule requests
  remain rejected while their existing qualification hold is unresolved.
- Added explicit runtime procedure grants and one bounded actor/payload-digest
  import audit in the same transaction. No path placeholders or JSONB state,
  new dependency, backwards-compatibility work or architectural change. Removed
  the retired path-profile writer; parser helpers remain only for existing unit
  validation tests, not runtime import.
- Tests now require successful matching native import and a forced-dry-run
  unresolved version retaining its prior active head. Eight focused real-service
  tests passed across both authentication modes: matching import, disabled draft,
  exact association version-3 creation/re-import and a late overlap rollback
  leaving export and source bytes unchanged. An initially failing repeat branch
  exposed a CASE-expression syntax error; parenthesizing it fixed the branch
  without changing assertions. Subset aggregate coverage remains failing.
  Linux arm64 build, strict Clippy, TypeScript and secrets scans pass. Do not infer
  recovery, full workflow or release completion from these focused tests.
  Evidence uses `target/native-import-writer-` logs. Full UI testing reached
  75 passed, one failed and 103 not run: native exported-bundle re-import
  returned 400. The import fence incorrectly applied root-key grammar to
  catalog keys (including the built-in `safe_dry_run` policy); both compiler
  and preparation procedure now use the existing per-resource key contracts.
  A focused regression passes and still rejects underscore profile keys.
  The real-service retry now passes portable exported-bundle re-import in both
  authentication modes. Full `just ui-e2e` still ends at 75 passed, one failed
  and 103 not run: `include_local_paths=true` routes into the retired path-profile
  exporter and returns 500. ADR 521 keeps host paths outside portable exchange;
  ADR 557 separately preserves explicit local export without import authority.
  Evidence uses `target/catalog-key-` logs. The regression,
  Linux build, strict Clippy, TypeScript and changed-file secrets analysis pass;
  this is not full Sonar analysis or positive published coverage evidence.
  `just ci` remains failing at historical init identity equality (1739 current
  procedures versus frozen 1624); sealed runtime and denied-input checks passed.
  No frozen hashes were repinned and no quality criteria were relaxed.
  Reviewed root/Rust/data instructions; scoped data guidance distinguishes the
  writer from qualification. Rollback only this writer/audit delta and preserve
  integration changes. Next: replace the retired local-path export reader, then
  authenticated UI save/restart/dry-run proof; full release gates remain open.

### Native Local Export Checkpoint

- Preserved ADR 557's explicit local-path export. Replaced the retired path-profile
  exporter with a bounded native configuration snapshot and referenced path
  diagnostics, read in one read-only repeatable-read transaction through a
  restricted procedure. Unresolved bindings retain null paths. The explicit
  `revaer.media.local_snapshot` kind and typed diagnostic array are not accepted
  by portable import; ordinary portable export remains path-free.
- Real-service retries in both authentication modes now pass local export,
  import rejection and unchanged portable/source assertions. The broader test
  subsequently fails on an obsolete profile PATCH expecting 200 instead of the
  native contract's 405. Seven export/compiler unit tests, Linux build, strict
  Clippy and TypeScript pass. This does not prove the complete UI milestone.
  Evidence: `target/local-export-` logs. The current full CI run still fails at
  frozen init identity equality (1741 current statements versus 1624); no repin.
- No new dependency, migration or architecture substitution. Reviewed root,
  Rust/data instructions and ADR 557's local/portable authority boundary; scoped
  data instructions now preserve that distinction. No new metric or path-bearing
  log. Rollback only this exporter/procedure delta, preserving other integration
  work. No full Sonar coverage, package, push or merge completion is claimed.

### Authenticated Workflow Milestone

- `just test-media-operator-workflow` passed against the real Linux service:
  authenticated browser policy/profile/association saves, immutable replacement
  and stale-edit rejection, real service-process restart, persisted dry-run plan
  and unchanged original media. The same run proved cancellation preserved the
  original, explicit UI retry completed, and verified replacement probed as H.264.
  Evidence: `target/service-restart-operator-workflow.log` and positive diagnostic
  JavaScript LCOV under `coverage/js/media-operator/`; not published Sonar coverage.
- The initial container-restart attempt failed association readiness. ADR 557
  includes mount identity and requires explicit rebinding after changed
  attestation. The milestone now restarts the actual service process through the
  existing fixture hook without replacing its attested namespace. No production
  fence changed. Container-remount/rebinding recovery remains unqualified and
  included release work, not implicitly waived by this passing milestone.
- Scoped UI guidance distinguishes process restart from remount/package proof.
  No dependency or architecture change. Full `just ui-e2e`, strict published
  Sonar coverage, shipping Linux packages and PR gates remain incomplete.
  Latest full `just ui-e2e`: 75 passed, one failed, 103 not run, at retired profile
  PATCH returning 405 versus an obsolete 200 expectation; aggregate coverage fails.
  Evidence: `target/local-export-full-ui.log`. Latest `just ci` failure is the
  historical init check above, not a runtime baseline failure.
  Next: reconcile obsolete profile-PATCH workflow tests with native immutable
  versions and association authority, preserving scheduler/watcher feature scope.

### Native Job Inspection Checkpoint

- The broader manual workflow now uses complete fenced profile replacement and
  an exact native association pin. Catalog round-trip coverage remains intact;
  discovery uses the owned video-only target rather than pretending a video-only
  fixture satisfies mandatory audio. Watcher/scheduler positive activation tests
  remain separate, unskipped release requirements, not converted into held-state
  successes.
- The real-service retry exposed absolute paths in ordinary job responses.
  Added restricted operator get/list/recent procedures projecting paths relative
  to immutable intent roots; invalid snapshots fail instead of returning a host
  path. Worker filesystem paths and execution reads are unchanged. Public field
  documentation and scoped data instructions now state this authority boundary.
  No dependency, migration, architecture or quality-gate change.
- Both authentication modes pass the complete manual API workflow, including
  get/list/recent relative-path checks. Strict Clippy (including production panic
  rules), TypeScript and Linux build pass. Focused route coverage remains failing;
  the renewed real browser save/restart/dry-run/cancel/retry/H.264 replacement
  proof passes with the operator projection. `just ci` remains failing on frozen
  init equality (1749 current statements versus 1624), with runtime baseline and
  denied-input verification passed. Latest `just ui-e2e`: 76 passed, two failed,
  105 not run, plus failing aggregate coverage. Both failures are native watcher
  and scheduler activation returning `media_configuration_pending_contract` (409)
  instead of successful creation (201). An older manual test's absolute-path
  expectation was corrected to the same relative-path/no-host-prefix contract;
  that test now passes. No positive automation assertion was weakened. Evidence:
  `target/operator-job-paths-` logs. Broader diagnostic text/path shaping is not
  qualified by these three summary reads. No full Sonar or package claim.
- Reviewed root, Rust, data and UI guidance. Rollback only these operator read
  adapters/procedures; preserve other integration work. Next: implement the
  native watcher/scheduler qualification still demanded by positive tests.

### Current Init Policy Checkpoint

- Ordinary CI was still reconstructing the frozen 1,624-statement candidate
  from current feature SQL. Applied this ADR's approved feature-development
  boundary to that test without changing historical hashes or SQL semantics.
  Historical exact-delta verification remains intact outside that phase.
- Current init validation requires its packaged header and complete statements,
  and retains rejection of top-level transaction, database, system and timeout
  controls. Its 36 assertions pass, including 35 negative mutations. Fresh
  sealed-baseline verification and rejection of missing endpoints and owner
  login also passed in the owned CI fixture. Static checks do not qualify
  installation privileges, recovery, packages or the operator workflow.
- No operator-facing capability was delivered by this gate repair. Native
  automation still needs association-based background runtime integration;
  activation holds were not removed merely to satisfy positive API tests.
- Root, Rust, data and DevOps guidance reviewed; DevOps guidance now identifies
  the current-init test boundary. No dependency, required-check, workflow trigger
  or Sonar-criteria change. Rollback only the test selection and shared envelope
  validator; preserve the integration worktree and archived reference pins.
- Full `just ci` passed the repaired init test and then failed in the historical
  ingestion attribute-race fixture. A focused reproduction reports: "attribute
  race UPDATE event, retained row or exact stack changed". No application suite
  completion is claimed. Evidence: `target/current-init-ci.log` and
  `target/current-init-next-failure.log`. Do not expand that reference-fixture
  failure into another standalone database investigation.
- Full `just ui-e2e` remains 76 passed, two failed and 105 not run, with aggregate
  coverage failing. Both failures are native watcher/scheduler association
  activation returning 409 instead of 201. Evidence:
  `target/current-init-full-ui.log`. Formatting/instruction drift and diff checks
  pass. Changed-file secrets scanning passed; Sonar code analysis was skipped
  because Vortex is unavailable on the configured connection. No full Sonar,
  published coverage, package, push or merge claim. Next: native automatic
  discovery through the approved association admission path.

### Native Trigger Admission Checkpoint

- Schedule and watcher run routes now use the same native association admission
  as manual discovery, rather than an unconditional pending-contract response.
  The server selects the trigger; the stored procedure independently validates
  its closed token and rechecks that mode on the locked immutable association.
  Generation/version, scope, descriptor fingerprint, de-duplication, readiness
  and dry-run fences are preserved. Successful admission publishes queued events
  through the existing API event path. No dependency or architecture change.
- The Linux unit test passed all 24 trigger/mode combinations. Association
  creation/import activation holds remain unchanged until the background runtime
  uses native association authority and is qualified. This is shared admission
  implementation, not a claim that automatic discovery is delivered.
- Root, Rust and data guidance reviewed; data guidance now records transactional
  trigger authority. Rollback only this trigger parameter, its exact procedure
  signature/grant and caller wiring, preserving the rest of the worktree.
- The first real-service run caught SQL syntax error 42601 in the mode CASE
  condition; explicit parentheses fixed it. Both authentication modes then
  passed the complete manual API workflow, including native admission,
  de-duplication, relative get/list/recent paths and unchanged source media.
  Focused aggregate route coverage still fails; no positive assertion was
  weakened. Strict all-target/all-feature Clippy, production panic checks,
  TypeScript, formatting/instruction drift and Linux build pass. Changed-file
  secrets scanning passes; code analysis remains unavailable on the configured
  Sonar connection, not a clean code-quality or published-coverage result.
- Full `just ci` passed fresh sealed-baseline and denied-input checks, then
  failed in the historical ingestion-race fixture. Full `just ui-e2e` remains
  76 passed, two failed and 105 not run, plus failing aggregate coverage. Both
  failures are creation of native automatic associations returning 409 versus
  required 201. No activation assertion was removed or converted into a hold
  success. No push, package qualification, published Sonar coverage or merge.
  Evidence:
  `target/native-trigger-` logs. This API proof is real-service admission, not
  browser UI or background filesystem-event/scheduler qualification. Next:
  replace the background runtime's legacy profile authority with native
  associations before releasing automation activation holds.
- Sonar command: `just --command sonar analyze secrets crates/revaer-api/src/app/media.rs crates/revaer-api/src/http/handlers/media.rs crates/revaer-app/src/media.rs crates/revaer-app/src/media/source.rs crates/revaer-data/src/media/association_jobs.rs crates/revaer-data/init.sql tests/specs/api/media.spec.ts .github/instructions/revaer-data.instructions.md docs/adr/591-outside-in-first-release-path.md`.
  Secrets analysis ran and reported no issues in all nine files. This does not
  certify code quality or coverage; Vortex unavailability was verified in the
  preceding checkpoint and was not repeatedly probed without a connection change.

### Watcher Authority Boundary Checkpoint

- Removed retired `MediaProfileRow` input from the filesystem watcher adapter.
  Callers now provide explicit registration ids and roots; disabling or changing
  a registration removes its old native watcher. Event identity is named
  `registration_public_id`, not implicitly a legacy profile id. The current
  background caller is adapted without enabling native automatic associations.
- Added a real recursive Linux filesystem-event test, alongside existing event
  kind, callback, coalescing/capacity and registration tests. All five passed in
  the owned Linux fixture. This is an adapter proof, not automatic admission,
  media processing, service recovery or shipping-package qualification.
- Confirmed the approved DISC-1 boundary requires durable discovery progress and
  operator-selected schedule intervals, with no default cadence. Do not activate
  automation by merely pointing the old process-local scheduler at native rows.
  No architecture, numeric budget, dependency, schema or activation hold changed.
- Root and Rust guidance reviewed. Rollback these watcher input/event-id changes
  together with their caller and tests; preserve other worktree changes.
  Strict Clippy (including production panic checks), formatting/instruction drift
  and all five Linux watcher tests pass. Full `just ci` still fails in the
  historical ingestion-race fixture after fresh baseline/denied-input and
  current-init checks. Full `just ui-e2e` remains 76 passed, two activation
  failures and 105 not run, with aggregate coverage failing. No operator-facing
  automatic workflow was delivered. No gate or positive assertion changed.
  Evidence: `target/native-watch-roots-` logs.
  Sonar command: `just --command sonar analyze secrets crates/revaer-app/src/media_discovery_watcher.rs crates/revaer-app/src/media_discovery_runtime.rs docs/adr/591-outside-in-first-release-path.md`.
  All three files were scanned with no secret findings. Code quality analysis
  remains unavailable on the previously verified Sonar connection; no full
  scan, published coverage, package, push or merge claim.
  Next: native durable rescan/schedule configuration through the approved DISC-1
  state model, then its background association admission and recovery proof.

### Explicit Schedule Configuration Checkpoint

- Implemented authenticated conditional create/read for an explicitly selected
  minute/hour cadence on the exact immutable association version. There is no
  default interval. Approved bounds are 1-43,200 minutes or 1-720 hours. The
  normalized DISC-1 schedule state lives in the current init; runtime receives
  only procedure grants. Root-catalog locking precedes the association head lock,
  and stale/inactive heads, invalid input and duplicate creation fail without
  changes. Saving never enables automatic discovery.
- Added typed wire models, API documentation and client generation, boundary
  tests, and real-service API cases for both authentication modes. Four focused
  cases passed, including missing precondition, absent configuration, invalid
  quantities/units/fields, stale version, duplicate protection, complete readback,
  unchanged association modes and unchanged source bytes. The focused command
  still failed whole-suite route coverage; it is not a full gate pass.
- Extended the existing real browser/operator workflow with API cadence save
  before an actual service-process restart and identical readback afterward.
  `just test-media-operator-workflow` passed: authenticated UI configuration,
  restart-persisted cadence, dry-run plan with unchanged original, cancellation,
  explicit UI retry, verification and completed H.264 replacement. Cadence save
  itself was through the authenticated API, not a new schedule UI. This owned
  Linux diagnostic fixture is not shipping-package or crash-recovery proof.
- Strict all-target/all-feature Clippy and production panic checks, the two
  Linux cadence boundary tests, API-documentation regression test, TypeScript,
  formatting, instruction drift and diff checks passed. Full `just ci` still
  fails at the historical ingestion-attribute-race fixture's UPDATE diagnostic
  expectation. Full `just ui-e2e` is 78 passed, two automatic-admission failures,
  107 not run, and incomplete aggregate coverage. Its first launch was stopped
  for missing explicit fixture inputs; only the corrected run is counted.
- Evidence is under `target/schedule-config-`. Sonar command:
  `just --command sonar analyze --file crates/revaer-api-models/src/media_schedule.rs --file crates/revaer-data/src/media/schedules.rs --file crates/revaer-app/src/media/schedules.rs --file crates/revaer-api/src/http/handlers/media_schedules.rs --file crates/revaer-api/src/openapi.rs --file tests/specs/api/media-schedule.spec.ts --project VannaDii_Revaer`.
  Local checks reported no findings, but Vortex is unavailable and all six deep
  analyses were skipped; command failed. No full Sonar, positive published
  coverage, package, push or merge claim. No suppression or acceptance gate changed.
- Secrets command:
  `just --command sonar analyze secrets crates/revaer-api-models/src/media_schedule.rs crates/revaer-api-models/src/lib.rs crates/revaer-data/src/media/schedules.rs crates/revaer-data/src/media/mod.rs crates/revaer-data/init.sql crates/revaer-app/src/media/schedules.rs crates/revaer-app/src/media.rs crates/revaer-api/src/app/media.rs crates/revaer-api/src/http/handlers/media_schedules.rs crates/revaer-api/src/http/handlers/mod.rs crates/revaer-api/src/http/router.rs crates/revaer-api/src/openapi.rs docs/api/openapi.json tests/specs/api/media-schedule.spec.ts tests/support/media-operator-workflow.cjs .github/instructions/revaer-data.instructions.md docs/adr/591-outside-in-first-release-path.md`.
  All 17 files were scanned without findings. Owned fixture databases, containers,
  volumes, test media and browser captures were removed; fixture cleanup passed.
- Root, Rust, data and UI guidance reviewed; data guidance now records the
  explicit-cadence authority boundary. No new dependency. Roll back the schedule
  table/procedures/grants, Rust adapters/wire/routes, documentation and focused
  tests together; preserve other integration changes. Observability uses existing
  storage-origin logging and closed API problem codes, with no host paths.
- Next outside-in deliverable: operator schedule authoring/readback, followed by
  approved durable rescan/claim progression, native background admission and
  recovery. Initial create/read does not complete editing, activation or workers.

### Schedule UI Checkpoint

- Added explicit cadence selection to the saved-association UI, independent of
  manual mode. Quantity and unit start empty; the exact association version and
  conditional create fence go to the native procedure. Readback must match
  identity, version, quantity and unit. Unconfirmed writes preserve drafts and
  require reload. Authentication/context changes invalidate pending responses.
  Saved values are displayed, not defaulted. No activation or editing claim.
- Two draft/confirmation unit tests pass. The real authenticated browser saves
  cadence, preserves a draft after an actual competing save/409, reloads the
  persisted cadence and confirms both values after service-process restart.
  The same workflow passes dry-run source preservation, cancellation, explicit
  retry and verified H.264 replacement. Desktop/mobile controls were visually
  inspected and mobile bounds passed with the navigation drawer closed. First
  attempts found missing accessible labeling and implicit option selection;
  both were fixed without changing assertions. Evidence: `target/schedule-ui-`.
- Current `just ci` still fails the unchanged ingestion-race UPDATE diagnostic.
  Strict wasm Clippy fails on 329 existing UI findings; none points to the new
  schedule modules. This is a failed gate, not permission to suppress warnings.
  Formatting/instruction drift and diff checks pass. Full `just ui-e2e` is
  78 passed, two native automatic-admission failures, 107 not run and incomplete
  aggregate coverage. No test was skipped or assertion relaxed to obtain a pass.
- Sonar secrets command:
  `just --command sonar analyze secrets crates/revaer-ui/src/services/api.rs crates/revaer-ui/src/features/media/manual_discovery.rs crates/revaer-ui/src/features/media/mod.rs crates/revaer-ui/src/features/media/association_list_view.rs crates/revaer-ui/src/features/media/schedule.rs crates/revaer-ui/src/features/media/schedule_view.rs tests/support/media-operator-workflow.cjs .github/instructions/revaer-ui.instructions.md`.
  Eight files scanned without findings. Deep-analysis command:
  `just --command sonar analyze --file crates/revaer-ui/src/features/media/schedule.rs --file crates/revaer-ui/src/features/media/schedule_view.rs --file crates/revaer-ui/src/services/api.rs --file tests/support/media-operator-workflow.cjs --project VannaDii_Revaer`.
  Local checks passed, but all four deep analyses were skipped because Vortex
  remains unavailable. No full Sonar or positive published coverage claim.
- Root, Rust and UI guidance reviewed; UI guidance records the cadence boundary.
  No dependency or architecture change. Existing origin logging/problem codes
  remain authoritative; UI errors contain no paths or credentials. Roll back the
  selector, schedule UI/state, client conditional-create helper and tests together,
  preserving unrelated work. Next: editable cadence and durable native scan/claim
  progression before automatic activation, then complete recovery qualification.

### Schedule Editing Checkpoint

- Added conditional cadence editing through the authenticated UI, native API
  and restricted stored procedure. The existing microsecond `updated_at` is the
  strong revision fence; no new revision column, default cadence or activation
  was introduced. Edits retain the anchor, next due time and coalescing evidence.
  The shared writer is private; only guarded create/read/replace procedures are
  granted to the runtime. Stale writes return 412 and preserve the UI draft;
  reload is explicit and no automatic rebase or resubmission occurs.
- Four real-service API tests pass across both authentication modes, including
  missing/malformed fences, successful edit, stale rejection and unchanged source
  media. The focused command fails aggregate whole-suite route coverage, so it
  is not a full gate pass. Three model tests, two UI draft/confirmation tests,
  API-documentation regression, TypeScript, core all-target/all-feature Clippy,
  core production panic checks, formatting, instruction drift and diff checks pass.
- `just test-media-operator-workflow` passed using the real Linux diagnostic
  service: authenticated cadence edit, competing-writer draft preservation,
  explicit reload, service-process restart persistence, dry-run source preservation,
  cancellation, explicit UI retry and verified H.264 replacement. Desktop/mobile
  captures were inspected. Visual review caught a stale unit selector after
  reload; keyed refresh and exact browser assertions fixed it. An earlier run
  missed readiness while compilation waited for other builds; the deadline and
  assertions were not relaxed. Evidence: `target/schedule-edit-`.
- Full `just ci` fails the existing ingestion-attribute-race UPDATE diagnostic
  contract. Strict wasm Clippy still fails existing UI findings; none is in the
  schedule modules. Full `just ui-e2e` finished: 78 passed, two native automatic
  watcher/schedule admission failures (`media_configuration_pending_contract`),
  107 not run and no UI coverage output. The full gate failed; its terminal
  output and `target/js-coverage-tests/test-results/` retain test evidence.
  No new criteria, suppression, dependency, architecture, push or merge claim.
- Sonar secrets command: `just --command sonar analyze secrets crates/revaer-api-models/src/media_schedule.rs crates/revaer-data/src/media/schedules.rs crates/revaer-data/init.sql crates/revaer-app/src/media/schedules.rs crates/revaer-app/src/media.rs crates/revaer-api/src/app/media.rs crates/revaer-api/src/http/handlers/media_schedules.rs crates/revaer-api/src/http/handlers/media_schedule_preconditions.rs crates/revaer-api/src/http/handlers/mod.rs crates/revaer-api/src/http/dto/errors.rs crates/revaer-api/src/http/router.rs crates/revaer-api/src/openapi.rs docs/api/openapi.json crates/revaer-ui/src/features/media/schedule.rs crates/revaer-ui/src/features/media/schedule_view.rs crates/revaer-ui/src/services/api.rs tests/specs/api/media-schedule.spec.ts tests/support/media-operator-workflow.cjs .github/instructions/revaer-data.instructions.md .github/instructions/revaer-ui.instructions.md`.
  All 20 targets passed. The final selector/browser correction also passed
  `just --command sonar analyze secrets crates/revaer-ui/src/features/media/schedule_view.rs tests/support/media-operator-workflow.cjs`.
  Deep command: `just --command sonar analyze --file crates/revaer-api-models/src/media_schedule.rs --file crates/revaer-data/src/media/schedules.rs --file crates/revaer-app/src/media/schedules.rs --file crates/revaer-app/src/media.rs --file crates/revaer-api/src/app/media.rs --file crates/revaer-api/src/http/handlers/media_schedules.rs --file crates/revaer-api/src/http/handlers/media_schedule_preconditions.rs --file crates/revaer-api/src/http/handlers/mod.rs --file crates/revaer-api/src/http/dto/errors.rs --file crates/revaer-api/src/http/router.rs --file crates/revaer-api/src/openapi.rs --file crates/revaer-ui/src/features/media/schedule.rs --file crates/revaer-ui/src/features/media/schedule_view.rs --file crates/revaer-ui/src/services/api.rs --file tests/specs/api/media-schedule.spec.ts --file tests/support/media-operator-workflow.cjs --project VannaDii_Revaer`.
  Local checks reported no findings, but all 16 deep analyses were skipped
  because Vortex is unavailable; command failed. The final selector correction
  was submitted again with
  `just --command sonar analyze --file crates/revaer-ui/src/features/media/schedule_view.rs --file tests/support/media-operator-workflow.cjs --project VannaDii_Revaer`.
  Both deep analyses were skipped for the same unavailable service; command failed.
  No full Sonar or positive published coverage claim.
- Root, Rust, data and UI guidance reviewed; scoped data/UI guidance now states
  revision-fenced editing and pending-due preservation. Existing storage-origin
  logging and closed problem codes cover stale writes without host paths.
  Roll back the edit model/procedure/grant, API/client/UI and tests together,
  preserving the create/read milestone and unrelated changes. Next: approved
  durable native rescan/claim progression and background admission, then recovery
  and shipping-package qualification. Remove owned test media/resources before
  this turn ends; retain the active unfinished worktree.

### Native Rescan Publication Checkpoint

- Added DISC-1's normalized per-association-version rescan and reason rows.
  Authenticated association creation publishes `configuration_activated` in the
  same transaction as activation. The seven-reason registry is seeded by the
  same private routine in init and v0 setup/reset; first real-service tests
  caught the missing reset seed and it was fixed, not bypassed. Repeated reasons
  coalesce; sequence increments are checked and overflow rolls back. Runtime
  can read reasons but cannot call the publisher or mutate tables directly.
- This is activation-request publication, not automatic admission. Run/claim
  fencing, captured configuration-activation identity, directory epochs,
  candidate/checkpoint publication, clean-census satisfaction and worker wiring
  remain required. No request is marked satisfied by candidate processing.
  No default cadence, process-local persistence substitute or new dependency.
- The focused real-service test creates configuration through the authenticated
  API, checks coalescing/closed reasons/overflow/restricted grants, restarts the
  real service and compares retained state and unchanged source bytes. Both
  authentication-mode cases pass. The focused command still fails whole-suite
  route coverage, so it is not a gate pass. TypeScript, Linux application build,
  formatting, instruction drift and diff checks pass. Full `just ci` fails the
  existing ingestion-attribute-race UPDATE diagnostic contract. Full `just ui-e2e`
  finishes with 79 passed, two native automatic-admission failures
  (`media_configuration_pending_contract`), 108 not run and absent UI coverage.
  Neither full gate passes; no criteria were relaxed, commit pushed or PR merged.
  Evidence: `target/rescan-`.
- Sonar commands:
  `just --command sonar analyze secrets crates/revaer-data/init.sql tests/specs/api/media-rescan.spec.ts .github/instructions/revaer-data.instructions.md` and
  `just --command sonar analyze --file crates/revaer-data/init.sql --file tests/specs/api/media-rescan.spec.ts --project VannaDii_Revaer`.
  Secrets passed; both deep analyses were skipped because Vortex is unavailable.
  The deep command failed, with no full scan or published coverage claim.
- Root/Rust/data guidance reviewed; data guidance records the activation request
  and remaining claim boundary. Existing API storage-origin diagnostics remain
  authoritative; no metric gains path/principal labels. Roll back the activation
  publication call, normalized tables/private routines/read grant and focused
  test together while preserving the prior workflow. Next: durable run ownership
  and bounded frontier execution, then native automatic admission.

### Discovery Claim-Slot Checkpoint

- Added normalized execution slots under DISC-1's approved capacity-row option:
  two deployment slots, with unique owned instance, source attestation and policy
  keys. Claims lock catalog/source/profile/association authority before capacity,
  capture the rescan high-water sequence and increment a durable generation.
  The 20-second server lease rejects renewal after expiry; expiry never releases
  capacity. Explicit release checks owner and fence and is permitted for cleanup
  after expiry, but its future worker caller must first establish quiescence.
- First real-service restricted-runtime tests pass in both authentication modes:
  idempotent live claim, contention, renewal, expired-owner rejection, retained
  capacity, explicit release, increasing replacement fence and stale-release
  rejection. The follow-up-trigger rerun passes in both authentication modes:
  an existing claim keeps sequence 8, while its replacement captures sequence 9.
  Focused commands fail aggregate route coverage, not these assertions. Full
  `just ci` fails the existing ingestion-race UPDATE diagnostic. Full `just ui-e2e`
  finishes with 79 passed, two native automatic-admission failures, 108 not run
  and absent UI coverage. Both full gates fail. TypeScript, Linux build, formatting and
  instruction drift pass. Evidence: `target/discovery-claim-`.
- These are database ownership primitives, not an integrated native scanner or
  physical settlement proof. Durable run snapshots/admission policy, fair
  scheduling, unknown-commit receipt readback, complete frontier epochs, atomic intent/checkpoints and clean
  census remain required; no automatic-admission hold was removed. No new
  dependency, migration, backward-compatibility work, push or merge.
- Root/Rust/data guidance reviewed; scoped data guidance records the ownership
  boundary. Existing closed storage errors report expired/conflicting claims;
  no public paths or new metric labels were added. Roll back slots, private
  setup seed, claim/renew/release routines and exact runtime grants together,
  preserving activation requests. Next: bind these slots to durable run/frontier
  execution and a worker that releases only after its scanner is quiescent.
- Sonar commands:
  `just --command sonar analyze secrets crates/revaer-data/init.sql tests/specs/api/media-rescan.spec.ts .github/instructions/revaer-data.instructions.md` and
  `just --command sonar analyze --file crates/revaer-data/init.sql --file tests/specs/api/media-rescan.spec.ts --project VannaDii_Revaer`.
  Secrets passed; both deep analyses were skipped because Vortex is unavailable.
  The final test extension was submitted with
  `just --command sonar analyze secrets tests/specs/api/media-rescan.spec.ts` and
  `just --command sonar analyze --file tests/specs/api/media-rescan.spec.ts --project VannaDii_Revaer`.
  Its secrets scan passed, but deep analysis was skipped for the same unavailable
  service and failed. No full Sonar scan or positive published coverage is claimed.

### Candidate Loss Checkpoint (2026-10-02)

- A missing candidate parent beneath a still-valid retained root now produces
  expected candidate absence, like a deleted candidate file. Manual discovery
  records `media_discovery_source_unstable` and continues with remaining paths.
  Declared-root loss and other filesystem failures remain errors. This follows
  the operator-approved pragmatic file-loss behavior; no new dependency or
  database contract is introduced.
- Four real Linux API cases passed across both authentication modes: deleted
  file and deleted descendant parent, skipped outcome, remaining dry-run job
  admission/readback, and unchanged remaining source bytes. This is service
  admission evidence, not completed execution or package qualification. The
  focused command still fails whole-suite route coverage at teardown.
- Fourteen retained-directory tests passed, including missing parent versus
  lost declared root. TypeScript, scoped all-target Clippy, Linux app rebuild,
  production panic-lint Clippy, instruction drift, and diff checks passed.
- Instruction review: `AGENTS.md` and `rust.instructions.md`; the scoped Rust
  guidance now explicitly distinguishes candidate disappearance from root
  failure. No quality policy was relaxed. Rollback removes this classification
  and its focused regression test; it does not modify persisted state.
- Sonar commands:
  `just --command sonar analyze secrets crates/revaer-media-runtime/src/root_catalog/directory.rs crates/revaer-media-runtime/src/root_catalog/directory/tests.rs crates/revaer-app/src/bootstrap/root_catalog/native.rs tests/specs/api/media-source-loss.spec.ts .github/instructions/rust.instructions.md`
  and
  `just --command sonar analyze --file crates/revaer-media-runtime/src/root_catalog/directory.rs --file crates/revaer-media-runtime/src/root_catalog/directory/tests.rs --file crates/revaer-app/src/bootstrap/root_catalog/native.rs --file tests/specs/api/media-source-loss.spec.ts --project VannaDii_Revaer`.
  Secrets passed; all four deep analyses were skipped because Vortex is
  unavailable, so the latter command failed. No full Sonar or published
  positive-coverage pass is claimed. The record itself passed
  `just --command sonar analyze secrets docs/adr/591-outside-in-first-release-path.md`.
- Full gates were attempted without relaxing criteria: `just ci` failed the
  existing attribute-race UPDATE diagnostic evidence check; `just ui-e2e`
  passed 81 cases, failed native watcher and schedule activation with
  `media_configuration_pending_contract`, and did not run 110 cases. Evidence:
  `target/source-loss-ci.log` and `target/source-loss-ui-e2e.log`.
  Automatic discovery, complete release verification, and merge remain open.

### Bounded Inventory Checkpoint (2026-10-02)

- The retained-directory fingerprint admission path now counts raw filename
  bytes before decoding/filtering, enforces the approved initial 1 MiB name
  budget alongside 4,096 entries, rejects invalid component names and duplicate
  enumeration results, and brackets enumeration with descriptor observations.
  Changed inventories produce the existing unstable-source outcome rather than
  stable membership. This is a live-tree observation, not an exclusive-root or
  adversarial-writer proof.
- Seven Linux fingerprint tests passed, including non-UTF-8 names, exact entry
  and byte boundaries, multibyte UTF-8 byte accounting, changed-directory
  rejection, and existing aggregate/source-preservation cases. Six applicable
  host cases passed. The host filesystem rejected creation of an invalid UTF-8
  filename; that case is explicitly Linux-only and passed in Linux, not waived.
  Evidence: `target/discovery-inventory-linux-tests.log` and
  `target/discovery-inventory-tests.log`. Production panic-lint Clippy passed.
- This is an exercised prerequisite, not completed automatic discovery. The
  current native path still lacks the approved bounded helper process, durable
  directory epochs/frontier consumption, complete aggregate-v1 identity and
  physical reservation settlement. Native watcher/schedule activation remains
  closed. No database investigation or new schema change was needed here.
- Policy review: root and Rust instructions, ADR 588 discovery contract and
  exact values; Rust guidance now requires byte accounting and stable complete
  inventories. No dependencies or criteria exceptions. Existing diagnostics
  carry unstable-source failures. Rollback removes the inventory checks and
  regression cases without changing persisted state.
- Sonar commands:
  `just --command sonar analyze secrets crates/revaer-app/src/media_discovery_fingerprint.rs crates/revaer-app/src/media.rs .github/instructions/rust.instructions.md`
  and
  `just --command sonar analyze --file crates/revaer-app/src/media_discovery_fingerprint.rs --file crates/revaer-app/src/media.rs --project VannaDii_Revaer`.
  Secrets passed; deep analysis remains unavailable through the configured
  Vortex connection. No full Sonar or positive published coverage is claimed.
- Four real Linux authenticated API regressions passed with the inventory
  changes; the focused run still failed whole-suite route coverage at teardown.
  Scoped all-target Clippy, production panic-lint Clippy, formatting, instruction
  drift and diff checks passed. Full `just ci` again failed the attribute-race
  UPDATE diagnostic check. Full `just ui-e2e` passed 81 cases, failed the two
  native watcher/schedule activation cases with
  `media_configuration_pending_contract`, and left 110 cases unrun. Evidence:
  `target/discovery-inventory-real-api.log`, `target/discovery-inventory-ci.log`
  and `target/discovery-inventory-ui-e2e.log`. No push or merge occurred.
- Next concrete implementation: bounded read-only helper-process execution
  through the retained-root admission path, then durable directory-epoch/frontier
  consumption. Preserve activation closure until the approved native worker and
  its required evidence exist. The full goal remains incomplete.

### Descriptor Helper Checkpoint (2026-10-02)

- Implemented ADR 588's same-binary `--media-fingerprint-helper` entry before
  Tokio/database bootstrap. The helper accepts a closed request of at most
  1 KiB, immutable member observations, request UUID/generation and the caller's
  nonzero absolute monotonic deadline. It reads only inherited read-only regular
  member/cancellation descriptors; there is no pathname, native-program or
  database input. Cancellation-pipe EOF also cancels work.
- Member reads use 64 KiB heap chunks, never exceed the observed reservation
  length, reject premature EOF or changed descriptor observations, and check
  cancellation/deadline before reads and terminal success. Progress is bounded
  to at most one <=32-byte record per five seconds; terminal output is <=1 KiB.
  This is a member SHA-256, not an aggregate-v1 identity or production admission.
- Real same-binary process cases passed on Linux and host: complete multi-chunk
  digest and original preservation; cancellation, expired absolute deadline,
  changed observation and writable-descriptor rejection; malformed/unknown,
  zero-deadline/generation, oversized requests and over-hard-limit observations
  rejected before normal bootstrap. Two reader tests passed on both platforms,
  covering mid-read cancellation and growth/shrink without unreserved reads.
  Four ordinary-entry regression tests passed. Evidence: final helper test logs,
  `target/fingerprint-reader-tests.log` and
  `target/fingerprint-reader-linux-tests.log`.
- This does not prove supervised KILL/reaping, OS containment, durable quota
  reservation/registration, late-result rejection by the parent, physical
  settlement, complete aggregate membership or automatic discovery. Native
  automation remains closed. Next: connect this reader through the supervised
  reservation path; do not add another unconnected database subsystem.
- Dependency rationale: no new crate; existing `rustix` pipe/time features
  provide safe inherited cancellation handles and shared monotonic deadlines.
  Root/Rust instructions and the exact ADR 588/589 helper contract were reviewed;
  Rust guidance now records the before-bootstrap boundary and remaining gates.
  No approval or criterion was invented. Rollback removes helper dispatch,
  reader/tests and feature additions, retaining ordinary startup and persisted
  data. Typed terminal reasons supply bounded failure observability.
- Sonar commands:
  `just --command sonar analyze secrets Cargo.toml crates/revaer-app/src/lib.rs crates/revaer-app/src/main.rs crates/revaer-app/src/bootstrap.rs crates/revaer-app/src/bootstrap/fingerprint_helper.rs crates/revaer-app/src/media_fingerprint_reader.rs crates/revaer-app/tests/fingerprint_helper.rs .github/instructions/rust.instructions.md`
  and
  `just --command sonar analyze --file Cargo.toml --file crates/revaer-app/src/lib.rs --file crates/revaer-app/src/main.rs --file crates/revaer-app/src/bootstrap.rs --file crates/revaer-app/src/bootstrap/fingerprint_helper.rs --file crates/revaer-app/src/media_fingerprint_reader.rs --file crates/revaer-app/tests/fingerprint_helper.rs --project VannaDii_Revaer`.
  Secrets passed; all seven deep analyses were skipped because Vortex is
  unavailable. No full Sonar or positive published coverage is claimed.

- Final helper validation passed four real-process cases and four ordinary
  entry cases on both Linux and host. Setup failures now emit bounded stdout
  evidence rather than attempting writes to the read-only cancellation channel;
  failure of the result channel exits explicitly without panic-based reporting.
  Final main/test changes were submitted with
  `just --command sonar analyze secrets crates/revaer-app/src/main.rs crates/revaer-app/tests/fingerprint_helper.rs`
  and
  `just --command sonar analyze --file crates/revaer-app/src/main.rs --file crates/revaer-app/tests/fingerprint_helper.rs --project VannaDii_Revaer`.
  Secrets passed; deep analysis was skipped for the same unavailable service.
- `just ci` failed the unchanged attribute-race UPDATE diagnostic evidence check.
  The first UI run referenced the previous Linux executable and is not current
  revision evidence. After correcting the harness to
  `revaer_app-84c7295b9847c55d`, full `just ui-e2e` passed 81 cases, failed native
  watcher/schedule activation with `media_configuration_pending_contract`, and
  left 110 cases unrun. Current evidence:
  `target/fingerprint-helper-ci.log` and
  `target/fingerprint-helper-current-ui-e2e.log`. Full release gates remain red;
  no production service completion, package qualification, push or merge is claimed.

### Supervised Member Reader Checkpoint

- Implemented the parent adapter required by automatic discovery: PID
  registration precedes the child's start acknowledgement; immutable reserved
  observations and byte counts are never refreshed after reservation. Changed
  sources, cancellation and expired deadlines cannot produce accepted digests.
- Parent output parsing is bounded and validates request/generation, byte count
  and closed terminal reasons. Acceptance requires completed output, successful
  child reaping, unchanged descriptor observations and current control state.
  Cancellation closes the control channel, then bounded settlement attempts
  KILL/reaping. An expired outer boundary or failed child observation returns
  retained ownership, not permission to reuse charged capacity or root barriers.
- Five helper and five supervisor process tests passed on host and Linux arm64,
  including start acknowledgement, registration rejection, cancellation,
  uncooperative and oversized-output workers, retained settlement ownership and
  source growth rejected before spawning. Evidence:
  `target/fingerprint-supervisor-tests.log` and
  `target/fingerprint-supervisor-linux-tests.log`. Production panic checks passed
  at `target/fingerprint-supervisor-panic-clippy.log`.
- This is a tested infrastructure prerequisite, not a newly delivered operator
  workflow. Production owner registration, durable byte reservation, automatic
  admission, aggregate membership and OS containment qualification remain
  incomplete. Next action: wire reservations and the owner registry into native
  discovery admission; do not enable the legacy scanner as a shortcut.
- No new dependency or architectural choice. Root/Rust guidance and ADRs
  588/589 were reviewed; Rust instructions clarify the child start barrier and
  immutable reservation observations. Typed failures and retained ownership
  provide bounded diagnostics. Rollback removes this adapter/start barrier and
  its tests without altering persisted state or ordinary service startup.
- Exact Sonar commands:
  `just --command sonar analyze secrets crates/revaer-app/src/bootstrap.rs crates/revaer-app/src/bootstrap/fingerprint_helper.rs crates/revaer-app/src/bootstrap/fingerprint_supervisor.rs crates/revaer-app/src/media_fingerprint_reader.rs crates/revaer-app/tests/fingerprint_helper.rs crates/revaer-app/tests/fingerprint_supervisor.rs .github/instructions/rust.instructions.md docs/adr/591-outside-in-first-release-path.md`
  and
  `just --command sonar analyze --file crates/revaer-app/src/bootstrap/fingerprint_supervisor.rs --file crates/revaer-app/src/bootstrap/fingerprint_helper.rs --file crates/revaer-app/src/bootstrap.rs --file crates/revaer-app/src/media_fingerprint_reader.rs --file crates/revaer-app/tests/fingerprint_helper.rs --file crates/revaer-app/tests/fingerprint_supervisor.rs --project VannaDii_Revaer`.
  Secrets passed; all six deep analyses were skipped because Vortex remains
  unavailable. No full Sonar or positive published coverage is claimed.
- Final all-target Clippy, formatting, instruction-drift and diff checks passed.
  `just ci` failed the existing attribute-race UPDATE diagnostic assertion
  (`target/fingerprint-supervisor-ci.log`). Current-revision full `just ui-e2e`
  passed 81 cases, failed native watcher/schedule activation with HTTP 409
  `media_configuration_pending_contract`, and left 110 cases unrun; aggregate
  coverage remains incomplete (`target/fingerprint-supervisor-ui-e2e.log`).
  These failures remain required fixes, not approval or quality-gate waivers.
  No package qualification, remote checks, push or merge is claimed.

### Helper Registration Identity Checkpoint

- The approved owner registration requires PID=PGID. Parent spawning now
  creates a separate helper process group with the existing safe standard
  library API, before the registration callback and start acknowledgement.
  It no longer inherits the application group. No dependency or design change.
- All five supervisor cases passed on host and Linux arm64, including an actual
  `getpgid` observation inside registration. Evidence:
  `target/fingerprint-group-host-tests.log` and
  `target/fingerprint-group-linux-tests.log`. All-target Clippy passed.
- This fixes a prerequisite, not production owner registration or automatic
  discovery. Remaining native admission dependencies are the actual owner
  protocol, durable fingerprint reservations and fenced traversal. Do not
  substitute the legacy scanner or a permissive registration callback.
  Existing failing full CI/UI evidence above is retained; the isolated helper
  spawn change does not repair those failures or certify new full-gate success.
- Root, Rust and data instructions plus the approved lifecycle identity
  contract were reviewed. No instruction drift or new approval was found.
  Registration failure remains a typed rejection with owned settlement;
  rollback removes this spawn change and its identity assertion without
  altering stored state. Test sources remain unchanged and are deleted after
  validation.
- Exact Sonar commands:
  `just --command sonar analyze secrets crates/revaer-app/src/bootstrap/fingerprint_supervisor.rs crates/revaer-app/tests/fingerprint_supervisor.rs`
  and
  `just --command sonar analyze --file crates/revaer-app/src/bootstrap/fingerprint_supervisor.rs --file crates/revaer-app/tests/fingerprint_supervisor.rs --project VannaDii_Revaer`.
  Secrets passed. Both deeper analyses were skipped because Vortex is
  unavailable; no clean deeper analysis or published coverage is claimed.

### Hash Registration Handshake Checkpoint

- Supervisor registration now supplies the typed helper identity and unchanged
  absolute cutoff. Added the exact approved 64-byte opcode-3 encoding and
  closed ACK validation, with no duration selection or clock renewal.
- Two codec cases and six supervisor process cases passed on host and Linux
  arm64; late registration cannot start the expired helper. Evidence:
  `target/hash-registration-{host,linux}-{codec,process}.log`. All-target
  Clippy passed. No new dependency or architectural choice.
- Actual owner transport/registry and durable fingerprint admission remain
  incomplete. Codec/parent tests are not owner-service or operator-workflow
  completion. Next: implement the approved owner control/registry boundary.
- Root/Rust instructions and ADR 588/589 were reviewed; Rust guidance and
  ADR 589 now distinguish codec support from owner enforcement. Existing
  typed rejection/settlement diagnostics remain; rollback removes the codec
  and typed registration callback together without changing persisted state.
- Sonar commands:
  `just --command sonar analyze secrets crates/revaer-app/src/bootstrap.rs crates/revaer-app/src/bootstrap/hash_registration.rs crates/revaer-app/src/bootstrap/fingerprint_supervisor.rs crates/revaer-app/tests/fingerprint_supervisor.rs .github/instructions/rust.instructions.md docs/adr/589-hash-helper-registration-deadline.md`
  and
  `just --command sonar analyze --file crates/revaer-app/src/bootstrap.rs --file crates/revaer-app/src/bootstrap/hash_registration.rs --file crates/revaer-app/src/bootstrap/fingerprint_supervisor.rs --file crates/revaer-app/tests/fingerprint_supervisor.rs --project VannaDii_Revaer`.
  Secrets passed; all four deeper analyses skipped because Vortex is unavailable.
  No full Sonar or positive published coverage is claimed.

### Owner Control Transport Checkpoint

- Added the approved two anonymous nonblocking control pipes: per-descriptor
  `_PC_PIPE_BUF` validation, bounded Linux capacity, close-on-exec endpoints,
  one 64-byte write and one 64-byte partial receive buffer. Full pipes, short
  writes and partial EOF return explicit failures without an outbound queue.
- Three real-pipe cases and seven supervisor process cases passed on host and
  Linux arm64. A scripted owner peer ACKs registration through actual pipes;
  a wrong nonce rejects and settles the real helper, preserving source bytes.
  Evidence: `target/owner-control-{host,linux}-{tests,process}.log`.
  This is component evidence, not complete owner-service or operator E2E proof.
- Dependency rationale: promote existing graph version `nix` 0.29 with only
  `fs` enabled to use its safe `fpathconf` wrapper. `std`/`rustix` do not supply
  the required per-descriptor check; no authored unsafe code or additional
  package version was introduced. Existing unrelated lock changes are retained.
- The prior hash-registration CI run was interrupted without a terminal result;
  its owned container was removed after confirming its process handle was gone.
  Its UI run failed the two automation-activation cases (81 passed, 110 unrun,
  missing UI coverage). It is not complete release verification.
- Root/Rust/FFI instructions and the exact approved lifecycle contract were
  reviewed. Rust guidance now records transport bounds and qualification limits.
  Typed I/O failures provide diagnostics; rollback removes the adapter, exports,
  direct dependency and tests together without altering stored data.
- Next: wire the actual same-binary owner and its registry/HELLO state. Admission
  stays closed until lifecycle, identity and reservation enforcement is proven.
- Sonar commands:
  `just --command sonar analyze secrets Cargo.toml Cargo.lock crates/revaer-app/Cargo.toml crates/revaer-app/src/bootstrap.rs crates/revaer-app/src/bootstrap/owner_control.rs crates/revaer-app/tests/fingerprint_supervisor.rs .github/instructions/rust.instructions.md`
  and
  `just --command sonar analyze --file crates/revaer-app/src/bootstrap.rs --file crates/revaer-app/src/bootstrap/owner_control.rs --file crates/revaer-app/tests/fingerprint_supervisor.rs --project VannaDii_Revaer`.
  Secrets passed; all three deeper analyses skipped because Vortex is unavailable.
  No full Sonar or positive published coverage is claimed.
- Final transport errors permanently fail the endpoint; subsequent reads/writes
  cannot retry or reopen admission. The updated three pipe cases passed on both
  platforms. Final source checks used
  `just --command sonar analyze secrets crates/revaer-app/src/bootstrap/owner_control.rs`
  and
  `just --command sonar analyze --file crates/revaer-app/src/bootstrap/owner_control.rs --project VannaDii_Revaer`;
  secrets passed and deeper analysis skipped for the same service limitation.
- Required gate evidence: `target/owner-control-ci.log` failed the existing
  attribute-race UPDATE diagnostic assertion; `target/owner-control-ui-e2e.log`
  passed 81 cases, failed watcher/schedule activation with HTTP 409
  `media_configuration_pending_contract`, left 110 cases unrun and lacked UI
  coverage. These failures remain required fixes. No push/merge, published
  coverage or package qualification is claimed.

### Hash Owner State Checkpoint

- Implemented the approved HELLO/hash registration and settlement state in
  `bootstrap/hash_owner_state.rs`. Admission requires the exact session ACK
  and injected identity validation. The fallibly allocated 128 KiB registry
  includes the application and retains child slots through trusted settlement
  and the peer's verified-record ACK. Expiry, failed validation and protocol
  failures cannot release ownership or reopen admission. No dependency added.
- Six state tests passed on macOS and Linux arm64, including registry capacity,
  64 unacknowledged settlements, malformed/reused identities and draining.
  Seven real-helper process cases passed on both platforms; the pipe fixture
  now uses the stateful owner peer and checks actual PGID before registration
  ACK. Synthetic start ticks and injected settlement validators remain unit
  fixtures, not kernel identity or physical-settlement qualification.
- Evidence: `target/hash-owner-state-{host,linux}-tests.log`,
  `target/hash-owner-state-{host,linux}-process.log` and
  `target/hash-owner-state-final-clippy.log`. Tests exercise the dirty integration
  tree at `f4b80bf76043c03091a1860c5c4b3de33ed256fe`; no new commit or merge.
- Motivation: this is a prerequisite for bounded fingerprint registration in
  automatic discovery, not a replacement for that operator deliverable.
  The actual same-binary owner/controller, kernel identity validation, trusted
  settlement receipt chain and durable admission wiring remain incomplete.
- Observability uses explicit typed I/O failures; no diagnostic-path I/O is
  added. Rollback removes this component, exports and stateful test peer
  together without altering persisted state or weakening existing gates.
- Stale-policy review: root, Rust and Sonar instructions were checked. Rust
  guidance now distinguishes the tested ledger from actual owner qualification;
  no architecture, approval boundary or gate was changed.
- Next: connect the real owner lifecycle and registration authority, then
  finish watcher/schedule activation through the authenticated workflow.
- Final local checks: all-target and panic/unwrap/expect Clippy, formatting,
  instruction drift and diff checks passed. `just ci` failed the existing
  `ingestion_attribute_race.rb:355` UPDATE diagnostic assertion.
  `just ui-e2e` passed 81 cases, failed watcher/schedule activation with
  HTTP 409 `media_configuration_pending_contract`, left 110 cases unrun and
  lacked UI coverage. Evidence: `target/hash-owner-state-ci.log` and
  `target/hash-owner-state-ui-e2e.log`. No push, package or merge claim.
- Sonar commands:
  `just --command sonar analyze secrets crates/revaer-app/src/bootstrap.rs crates/revaer-app/src/bootstrap/hash_owner_state.rs crates/revaer-app/tests/fingerprint_supervisor.rs .github/instructions/rust.instructions.md docs/adr/591-outside-in-first-release-path.md`
  and
  `just --command sonar analyze --file crates/revaer-app/src/bootstrap.rs --file crates/revaer-app/src/bootstrap/hash_owner_state.rs --file crates/revaer-app/tests/fingerprint_supervisor.rs --project VannaDii_Revaer`.
  Secrets passed; all three deeper analyses skipped because Vortex is unavailable.
  No full scan or positive published coverage is claimed.

### Linux Hash Identity Checkpoint

- Added `bootstrap/hash_identity.rs` to implement the approved pre-ACK live
  identity check. It retains the application's PID/start identity, UID and PID
  namespace, verifies direct child parentage and PID=PGID, bounds proc reads,
  and rechecks observations before accepting the supplied start ticks. No
  dependency added; no proc reads enter shutdown/deadline handling.
- The real helper pipe test now encodes actual Linux start ticks and calls
  this authority before registration ACK. Three Linux identity tests and all
  seven real-helper process cases passed, including wrong group and wrong
  start identity rejection. Evidence: `target/hash-identity-linux-tests.log`
  and `target/hash-identity-linux-process.log`. The parser fixture initially
  had an incorrect field count; it was corrected to the documented Linux
  field position before the successful rerun.
- These observations verify same-namespace live identity, not private PID1
  deployment, owner shutdown, trusted settlement receipts or task admission.
  The owner controller and automatic watcher/schedule workflow remain pending.
- Motivation: replace synthetic identity in the discovery prerequisite with
  the approved real check. Typed I/O errors retain explicit rejection reasons;
  rollback removes the adapter, exports and Linux test integration together,
  without changing persisted state. Root/Rust instructions and ADR 588's
  lifecycle contract were reviewed; Rust guidance records the admission-only
  I/O boundary. No new decision or relaxed gate was introduced.
- Tests used the dirty integration tree at
  `f4b80bf76043c03091a1860c5c4b3de33ed256fe`, diagnostic Linux arm64 Rust image
  `sha256:5e2214abe154fe26e39f64488952e5c991eeed1d6d6da7cc8381ae83927f0cfc`.
  This is not shipping-package qualification.
- Linux all-target lint and the separate production panic/unwrap/expect lint
  passed at `target/hash-identity-linux-final-clippy.log`; host all-target lint,
  formatting and instruction drift passed. Linux lint also exposed used
  underscore-prefixed root-catalog fields; they were renamed without behavior
  changes. An intermediate command incorrectly applied production-only panic
  rules to test targets and failed on existing test helpers; the final commands
  follow the root's distinct test and production contracts, without suppression.
- `just ci` failed the unchanged attribute-race UPDATE diagnostic assertion;
  `just ui-e2e` passed 81, failed the two watcher/schedule HTTP 409 activation
  cases, left 110 unrun and produced no UI coverage. Evidence:
  `target/hash-identity-ci.log` and `target/hash-identity-ui-e2e.log`.
  These full gates preceded the behavior-preserving final lint fixes; neither
  failed run certifies this revision. Owned databases, containers and test
  media were removed; `just clean-test-fixtures` passed. No push or merge.
- Sonar commands:
  `just --command sonar analyze secrets crates/revaer-app/src/bootstrap.rs crates/revaer-app/src/bootstrap/hash_identity.rs crates/revaer-app/tests/fingerprint_supervisor.rs .github/instructions/rust.instructions.md docs/adr/591-outside-in-first-release-path.md`
  and
  `just --command sonar analyze --file crates/revaer-app/src/bootstrap.rs --file crates/revaer-app/src/bootstrap/hash_identity.rs --file crates/revaer-app/tests/fingerprint_supervisor.rs --project VannaDii_Revaer`.
  Final source checks after lint fixes:
  `just --command sonar analyze secrets crates/revaer-app/src/bootstrap/hash_identity.rs crates/revaer-app/src/bootstrap/root_catalog/native.rs`
  and
  `just --command sonar analyze --file crates/revaer-app/src/bootstrap/hash_identity.rs --file crates/revaer-app/src/bootstrap/root_catalog/native.rs --project VannaDii_Revaer`.
  Secrets passed; deeper analysis skipped because Vortex is unavailable.
  No full scan, positive published coverage or service completion is claimed.
- Next: actual same-binary owner launch/control integration and trusted
  settlement authority, then durable automatic discovery. This adapter is
  only the approved admission-phase identity prerequisite.

### Owner Application Pipe Transfer Checkpoint

- Implemented the approved two-pipe application exec transfer in
  `bootstrap/owner_control.rs`, preserving stdout/stderr for existing logs.
  Transfer consumes the parent's application ends; adoption validates direction,
  nonblocking FIFO identity and pipe bounds, restores close-on-exec ownership
  and closes originals. Private argument binding is caller-supplied, not an
  environment-selected endpoint. Proc fd aliases duplicate anonymous pipes
  during bootstrap only; no pathname-based control channel was added.
- Five Linux transport cases passed, including real child exec, separate
  stdout, record exchange, EOF after child exit, rejection of invalid handles
  and no control-handle inheritance into a subsequent exec. The ignored child
  fixture is explicitly executed and asserted by the parent case; it is not
  an unexecuted acceptance test. Evidence:
  `target/owner-inheritance-linux-tests.log`. The same library test executable
  exercises exec transfer, not the shipping service's PID1/application mode.
- Motivation: preserve existing application logging while connecting the
  approved independent supervisor. No new dependency or architecture decision;
  existing safe `nix` fcntl/close wrappers avoid authored unsafe ownership code.
  Transfer must occur before application threads; no inheritance queue or
  temporary environment selector is introduced. Typed I/O errors expose setup
  failure. Rollback removes the transfer/adoption methods and their tests
  together without changing persisted state.
- Root/Rust/FFI instructions and ADR 588's lifecycle contract were reviewed.
  Rust guidance now records the bootstrap-only inheritance boundary. Actual
  service mode dispatch, signal handling, deadline enforcement and settlement
  authority remain incomplete; automatic discovery is not enabled.
- Tested the dirty integration tree at
  `f4b80bf76043c03091a1860c5c4b3de33ed256fe` using the same pinned diagnostic
  Linux arm64 Rust image as the preceding checkpoint. No package qualification,
  newly delivered operator capability, push or merge is claimed.
- Host all-target lint, Linux all-target/production panic lint, formatting,
  instruction drift and diff checks passed. `just ci` failed the unchanged
  attribute-race UPDATE diagnostic assertion; `just ui-e2e` passed 81 cases,
  failed watcher/schedule activation, left 110 unrun and lacked UI coverage.
  Evidence: `target/owner-inheritance-{host,linux}-clippy.log`,
  `target/owner-inheritance-ci.log` and `target/owner-inheritance-ui-e2e.log`.
  Full gates preceded the final equivalent boolean/import lint fixes and
  neither failed run certifies this revision.
- Sonar commands:
  `just --command sonar analyze secrets crates/revaer-app/src/bootstrap/owner_control.rs .github/instructions/rust.instructions.md docs/adr/591-outside-in-first-release-path.md`
  and
  `just --command sonar analyze --file crates/revaer-app/src/bootstrap/owner_control.rs --project VannaDii_Revaer`.
  Final source secrets check:
  `just --command sonar analyze secrets crates/revaer-app/src/bootstrap/owner_control.rs`.
  Secrets passed; deeper analysis skipped because Vortex is unavailable. No
  full Sonar or published positive coverage is claimed. Test media and owned
  databases/containers were removed; `just clean-test-fixtures` passed.
- Next: wire actual single-threaded service owner/application dispatch and
  HELLO session, then implement the approved stop and settlement authority.

### Attribute Race CI Fixture Repair

- The repeated CI failure was reproduced in the Ruby fixture-only test, not
  PostgreSQL. Its final-schema diagnostic fixture added an obsolete one-line
  offset: expected line 1730 versus actual routine line 1729. Removed that
  offset without changing SQL, the validator, exact stack matching or gates.
- Added negative regression coverage that shifts each ingestion stack line
  by one and must still fail exact validation. The focused fixture test passed:
  `just --command ruby scripts/tests/database-ingestion-attribute-race-test.rb`.
  This is test evidence only, not live database or service qualification.
- Motivation: unblock the canonical CI gate with a bounded fixture correction,
  not pursue legacy equivalence or new database architecture. No dependency,
  production behavior or observability change; diagnostics remain exact.
  Rollback restores the fixture offset and regression delta together. Root,
  data and DevOps instructions were reviewed; no policy change was needed.
- Evidence uses the dirty integration tree at
  `f4b80bf76043c03091a1860c5c4b3de33ed256fe`; focused output is retained in
  `target/attribute-race-focused.log`. Full gates are separately recorded below.

### Earlier Gate Evidence (Historical)

- `just ci` passed at `target/revaer-host-backed-ci-e31daedf8a15/gate.log`.
- Full `just ui-e2e` failed: schedule enablement returned HTTP 400 with
  `media_profile_filesystem_identity_required` instead of 200; 57 tests passed,
  one failed and 80 did not run. UI coverage was consequently absent. Evidence:
  `target/revaer-host-backed-ui-e2e-5ce47105232f/gate.log`.
- These runs exercised the unchanged runtime at local `f1a8624f` plus the
  preserved integration delta, not the proposed init path. No new Sonar scan,
  package qualification, remote check result or merge is claimed.
- Both owned validation databases were removed; `just clean-test-fixtures`
  passed. Documentation indexing, instruction drift and diff checks passed.
