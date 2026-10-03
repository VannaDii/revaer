# Complete profile version wire format

- Status: Accepted
- Date: 2026-09-15
- Operator approval: 2026-09-15, "ADRs 590 and 589 are approved under your recommendations."
- Supersedes: None; completes the unassigned profile representation in ADRs 521/557.
- Implementation status: Validated request/response DTOs and create-editor
  transport implemented; server persistence, replace/archive UI and release
  qualification remain incomplete.

## Approval Resolution

The operator approved the recommendation on 2026-09-15, including clear UI
presentation of operational eligibility and effective dry-run behavior. Test
disabled admission, conflicting edits and every combination of profile/policy
dry-run settings. The public-contract hold is released; implementation and
release qualification remain outstanding.

### Request Implementation, 2026-09-15

- Integrated `13082a69`: complete `ProfileVersionRequest` in the existing
  shared API models, exact keys/text, positive explicit versions, required
  safety booleans, omitted optional roots and rejection of null/unknown/duplicate
  fields. No new dependency, fallback, compatibility interface or database work.
- One bounded isolated worker authored only the DTO, tests and module exports.
  Reviewed the patch and integrated its commit; closed the worker and removed its
  clean completed worktree. No PR push or release qualification is claimed.
- Worker verification through `just --command cargo`: 12 unit tests, one doctest,
  scoped Clippy and formatting passed using installed Command Line Tools.
  These prove request representation, not HTTP size enforcement, membership,
  persistence, enabled admission, effective dry-run or concurrency behavior.

### Response Implementation, 2026-09-21

- Added the complete typed response with exact submitted fields, immutable
  version/lifecycle heads, UTC timestamps and ordered path-free root bindings.
  Validation preserves disabled active versions and prior active heads for
  drafts; it rejects mismatched keys, missing/extra bindings, incoherent
  readiness, unknown/duplicate fields and positional arrays. No new dependency.
- All 95 shared root/profile contract unit tests passed, including five new
  response tests. Strict scoped all-target/all-feature Clippy and formatting
  passed after correcting documentation lint findings. These are transport
  tests, not authenticated persistence, enabled admission or restart evidence.
- Root/Rust instructions reviewed; no rule changes or architectural deviations.
  Diagnostics omit submitted values. Rollback is the scoped unshipped model
  change, not modification of persisted immutable versions. Full CI/UI/Sonar
  gates remain outstanding as recorded in ADR 591.

### Create Editor Implementation, 2026-09-21

- Replaced the root-only placeholder with complete profile inputs, explicit
  target/policy versions and catalog-backed root selection. Defaults remain
  disabled/dry-run-only. Submit uses `If-None-Match: *`, preserves the draft on
  errors and confirms only an exact returned body with initial active/latest
  heads and a strong ETag. No fallback to the path-taking request is introduced.
- The canonical production UI build and scoped strict Clippy passed. All five
  route-controlled Chromium tests passed, including the complete submitted body,
  rejected-write draft preservation and desktop/mobile root controls. The
  focused browser command still failed global teardown because its API coverage
  files were absent; coverage enforcement was not disabled. Evidence:
  `target/milestone-profile-ui-1790012673-19605`. These mocked configuration
  responses do not prove database persistence, restart continuity or a dry run.
- No dependency or architecture change. Existing UI/root/Rust rules apply;
  operator drafts and bounded diagnostics remain intact. Next implement the
  approved server-side versioned save instead of claiming this UI is a usable
  end-to-end workflow.

### Create Request Size Boundary, 2026-09-21

- Applied the approved 1 MiB body bound to the profile collection route without
  changing authentication or conditional-write ordering. The real route rejects
  unauthenticated oversized requests before body handling; the guarded extractor
  test accepts exactly the bound and rejects one extra byte without relying on
  a Content-Length header. No dependency or public schema change.
- Focused API profile tests passed (25, including unrelated profile tests selected
  by the filter). This is request-boundary evidence, not profile persistence or
  an authenticated UI/service milestone. The route still uses the old request;
  complete immutable-version persistence remains the next implementation step.
- Reviewed root/Rust instructions; unchanged approval and quality gates apply.
  Scoped all-target no-default-feature Clippy passed with warnings denied.
  Rollback is removal of this scoped unshipped route limit. No observability
  change; no full CI/UI, Sonar quality or package qualification claimed.

### Profile Schema Implementation, 2026-10-01

- Operator explicitly authorized implementing ADR 557's profile schema:
  "Yes, this is explicitly authorized". Applied immutable version and logical
  root-binding tables, same-parent latest/active composite foreign keys,
  lifecycle/text/version bounds and update/delete rejection in the single init.
  No migrations, dependency, runtime table grant or deployment assertion added.
- Full-init PostgreSQL restricted-runtime fixture passed on
  `postgres@sha256:57c72fd2a128e416c7fcc499958864df5301e940bca0a56f58fddf30ffc07777`.
  It exercises head ownership, duplicate versions, field bounds, root-role and
  resolution constraints, immutability and denied direct runtime table access.
  Test input is synthetic schema evidence, not filesystem or service proof.
- Tested integration HEAD `f4b80bf76043c03091a1860c5c4b3de33ed256fe` plus the
  uncommitted schema/test changes. Scoped strict Clippy, formatting and secrets
  scanning passed. Sonar `verify` reported its command deprecated and Vortex
  unavailable; no quality-analysis or positive-coverage pass is claimed.
- Reviewed root, Rust and data instructions and accepted ADRs 521/557/590/591.
  No accepted design changed. The earlier automatic-review schema hold is
  released by the explicit authorization. Runtime save/read/replace/archive
  wiring and complete head/binding admission checks remain unfinished.
  Existing path-taking routes remain pending coordinated retirement; this does
  not qualify a usable profile editor, restart or dry-run milestone.
- No observability change. Rollback before release is reverting this unshipped
  schema delta; do not erase caller-owned databases or immutable history.
  Disposable database/container cleanup passed. Full CI/UI, strict Sonar with
  positive coverage, package and review gates remain outstanding.

### Save Implementation Checkpoint, 2026-10-01

- The operator explicitly authorized work in this worktree on 2026-10-01
  after confirming its parent is included in the project. Request filesystem
  escalation when the execution sandbox omits that project folder; do not
  ask the operator to repeat existing product approvals or change app settings
  as a substitute for requesting the available escalation. This records
  workspace authorization, not a change to architecture or quality gates.
- Corrected the schema display bound to the approved 1-128 UTF-8 bytes while
  preserving submitted whitespace. The older shared display validator trims
  and accepts 256 bytes, so it was not this profile contract's authority.
  Regression coverage includes preserved spaces and 128/130-byte multibyte names.
  The full-init restricted-runtime fixture passed again with this correction;
  source HEAD is unchanged and the tested delta remains uncommitted.
- Automatic approval review rejected the complete save patch. Nothing from
  that rejected patch was applied. Its stated concerns were procedure scope,
  existing path-column nullability, policy-version freezing and writer checks.
  This was an execution authorization hold, not a new architectural decision;
  the explicit operator authorization below supersedes it.
- The authorized change adds `media_profile_version_create_v1`
  with all ADR 590 fields and an explicit actor, and
  `media_profile_version_get_v1(uuid)` returning complete path-free normalized
  rows. Create runs in one serializable transaction, validates exact references
  and current-generation logical bindings, inserts version 1 and its children,
  and advances both heads. Failed validation rolls the entire write back.
- Permit path-free parents by removing the old source/output NOT NULL
  requirements and skipping the old path-overlap trigger for those parents.
  Freeze policy components when a profile version references them. Grant only
  the two new procedures through the existing sealed runtime allowlist; no
  direct table grant. Wire their typed service/API responses with the approved
  strong ETag. No new deployment assertion or proof of external exclusivity.
- Validate constrained-role create/read, exact round trips, disabled profiles,
  dry-run values, stale/unknown/forbidden roots, optional binding truth tables,
  duplicate-create rollback, immutable references and authenticated HTTP behavior.
  Legacy route retirement and full release qualification remain required.

### Atomic Save Authorization And Implementation, 2026-10-01

- Operator explicitly authorized the save patch's path-free nullable parent
  columns, atomic profile creation, freezing of referenced policy components,
  and runtime procedure grants under ADRs 557 and 590, and resumed single-agent
  work preserving every quality gate. This resolves the preceding write hold;
  it does not authorize new architecture or criterion relaxation.
- Implemented the serializable creator, immutable version and binding inserts,
  simultaneous latest/active heads even when disabled, paired nullable parent
  paths, and exact sealed procedure grant. Policy updates guard both old and new
  references against edits or moving a component away from a frozen policy.
- The data adapter reads the complete persisted representation in the write
  transaction, explicitly rolls back failed writes/reads, and retries only
  definitive serialization/deadlock failures at most twice. The service passes
  exact typed values and maps bounded validation/conflict codes without exposing
  SQL or submitted values.
- Passed the full-init restricted-runtime database test, including exact create
  and read, disabled active heads, missing references, unknown/forbidden roots,
  optional binding mismatches, failure rollback, duplicate preservation,
  path-free parent columns and policy freezing. Data strict lint also passed.
- Passed the service's bounded create-error mapping test and all three complete
  profile response tests. Strict API/app all-target lint, formatting and diff
  whitespace checks passed. No media was generated; `just clean-test-fixtures`
  removed ignored media fixture directories.
- Sonar secrets analysis ran successfully with no reported findings:

  ```sh
  just --command sonar analyze secrets crates/revaer-data/init.sql crates/revaer-data/src/media/profile_versions.rs crates/revaer-data/src/baseline/pool/tests/profile_creation.rs crates/revaer-data/src/baseline/pool/tests.rs scripts/tests/database-profile-version-create-fixture.sql crates/revaer-api/src/app/media.rs crates/revaer-app/src/media.rs docs/adr/590-profile-version-wire-completion.md
  ```

  This is secrets-only evidence. The previously recorded unavailable connected
  quality analyzer remains unresolved; it was not repeatedly retried unchanged.
  No scanner scope, coverage, quality profile or release gate was relaxed.
- Next action: coordinate the collection reader and authenticated POST cutover;
  the old collection assumes non-null filesystem paths and cannot list a new
  path-free parent. Public POST is not switched in this checkpoint. Then prove
  authenticated UI save, real service restart and dry-run source preservation.
  No operator-facing capability, full CI/UI gate, positive Sonar coverage or
  package qualification is claimed by these focused tests.

### Read-Only Persistence Checkpoint, 2026-10-01

- Worktree escalation succeeded after the operator explicitly directed it.
  Automatic review nevertheless rejected the complete save mutation again,
  citing nullability, policy freezing, grants and catalog admission. Nothing
  from that patch was applied; the rejection was not worked around piecemeal.
- Completed the unaffected `media_profile_version_get_v1(uuid)` and typed data
  adapter. They read existing immutable rows in one snapshot, preserve exact
  values and canonical binding order, and report current readiness separately
  from persisted resolution and profile enablement. Only the reader is granted
  through the sealed runtime allowlist; runtime table access remains denied.
- The full-init restricted-runtime test passed after the final row grouping:
  1 passed, 0 failed, 0 ignored. It covers exact draft values, unmapped bindings,
  unknown identity, disabled active heads, resolved bindings and stale catalog
  evidence without rewriting immutable history. This uses synthetic persisted
  roots, not filesystem attestation, authenticated HTTP or real-service evidence.
- Scoped formatting and strict Clippy passed; the initial excessive-booleans
  finding was fixed by grouping readiness, without suppressions. Secrets checks
  passed. `sonar verify` failed with a deprecation warning and unavailable
  Vortex; current `sonar analyze --depth DEEP` removed the deprecated invocation
  but still skipped quality analysis for all six reader files and failed because
  Vortex is unavailable. No Sonar quality or positive coverage pass is claimed.
- `just ci`, `just ui-e2e`, packages and PR checks are not qualified by this
  checkpoint. No source media was created. The next workflow action remains
  the approved configuration save, followed by restart and preserved-source
  dry-run planning; the write patch remains subject to the rejected escalation.
- Risk/rollback: the reader adds only read privileges and no execution authority.
  Revert the reader and its sealed allowlist entry if qualification fails; never
  delete immutable profile history to roll back a read adapter. No dependency
  was added. Reviewed root, Rust and data instructions; no gate was relaxed.

### API Read Checkpoint, 2026-10-01

- Connected the read-only procedure to the real media facade and authenticated
  profile item GET. Validate complete typed fields, identical repeated snapshot
  metadata, lifecycle/heads and exact ordered bindings before responding.
  Unknown profiles remain absent; query or corrupt-snapshot failures propagate
  as storage errors without exposing SQL diagnostics or fabricated defaults.
- Responses use the exact latest-version strong ETag and no-store. Item route
  middleware also applies no-store to authentication and extraction failures.
  Updated the GET OpenAPI reference, complete path-free schema and headers.
  Collection, create, replacement and archive cutover remain incomplete; there
  is no legacy-read fallback, and this intermediate revision is not releasable.
- Focused app assembler tests passed (3); focused API tests passed (8), including
  response/body fidelity, ETag fencing, unavailable-provider behavior and the
  OpenAPI contract. These are unit/handler tests, not real-service UI/restart
  evidence. The earlier restricted-database reader evidence is reused because
  its SQL/data adapter did not change in this checkpoint.
- Strict scoped API/app Clippy passed without new suppressions. Fixed its
  observed constant-constructor and root-identity-helper findings with const
  qualifiers, and borrowed snapshot rows rather than moving unused ownership.
  Formatting and secrets scans passed. Unavailable Vortex quality analysis was
  not repeatedly retried; no full CI, UI E2E or positive coverage pass is claimed.
- Observability remains one bounded origin log on read/validation failure, with
  no profile values or SQL diagnostics. No dependency was added. Revert this
  read-side wiring together with its OpenAPI changes if qualification fails;
  do not rewrite persisted versions. Root and Rust instructions were reviewed;
  no architecture, admission rule or quality gate was relaxed.
- The rejected save patch remains unapplied. Next: obtain the specifically
  requested write escalation, complete authenticated save, restart the real
  service and prove preserved source bytes through dry-run planning.

### Profile Create And Collection Cutover, 2026-10-01

- POST now consumes the complete validated ADR 590 request, calls the atomic
  creator, and returns the persisted path-free representation, 201, strong
  latest-head ETag and Location. Authentication precedes precondition/body
  handling; malformed bodies return bounded 400 problems and oversized bodies
  return bounded 413 problems. Collection and item responses retain no-store,
  including authentication failures. The retired path/retention create body is
  rejected rather than normalized into the new contract.
- GET collection now uses one bounded stored-procedure snapshot of complete
  latest profile representations. Pagination bounds parents, not child bindings,
  with default 50, maximum 200, key/public-identity ordering and one lookahead
  parent. Profile-only canonical URL-safe cursors use the `profiles_` resource
  prefix and the existing bounded key/UUID encoding. Root-catalog tokens cannot
  be replayed against the profile collection. SQL verifies cursor membership.
  Only the exact page procedure was added to the sealed runtime allowlist.
- Passed the full-init restricted-runtime database test with saved-profile
  collection membership, complete-parent pagination, no skips/repeats and
  invalid bounds/membership rejection. The cursor test, focused profile HTTP
  tests, complete-response/page tests and strict API/model/data/app lint passed.
  Regenerated the OpenAPI artifact through `just api-export`.
- The targeted secrets scan ran without findings on every source touched in this
  checkpoint. Its exact command is:

  ```sh
  just --command sonar analyze secrets crates/revaer-api-models/src/media_root_contract.rs crates/revaer-api-models/src/media_root_contract/profile_page.rs crates/revaer-data/init.sql crates/revaer-data/src/media/profile_versions.rs crates/revaer-data/src/baseline/pool/tests/profile_creation.rs crates/revaer-app/src/media/profile_versions.rs crates/revaer-app/src/media/profile_versions/tests.rs crates/revaer-app/src/media.rs crates/revaer-api/src/app/media.rs crates/revaer-api/src/http/handlers/media.rs crates/revaer-api/src/http/handlers/media_profile_representation.rs crates/revaer-api/src/http/router.rs crates/revaer-api/src/http/router/media_root_tests.rs crates/revaer-api/src/http/dto/errors.rs crates/revaer-api/src/openapi.rs
  ```

- Current failure/next action: `features/media/api.rs::fetch_profiles` and the
  dashboard's profile list still expect the retired path-based representation;
  their decoder cannot consume the complete version page. Replace that reader
  and obsolete path-based controls with the approved versioned UI. The separate
  typed create editor already targets this POST, but actual authenticated UI
  save, catalog startup, service restart and unchanged-source dry-run evidence
  remain pending. These are backend/boundary tests, not real service E2E proof.
- The complete release remains unqualified: no full `just ci`/`just ui-e2e`,
  positive Sonar coverage, applicable GitHub checks or package validation is
  claimed. No approval was invented and no gate or included feature was dropped.

### Versioned Dashboard Checkpoint, 2026-10-01

- Replaced the retired path-based profile reader and controls with complete
  validated version pages and feature-owned presentation state. The list shows
  exact text, logical bindings, latest/active heads, enablement and separate
  binding/destructive readiness. Confirmed creates refresh both readers without
  clearing an unsaved association draft. Loading and failed access clear stale
  list evidence; obsolete asynchronous completions are ignored.
- Removed the obsolete absolute-path create, partial toggles and path-preview
  controls. Their approved replacement capabilities are not declared complete:
  replace/archive, association persistence, discovery and automation remain
  required. No included feature, dependency or quality gate was dropped.
- Passed 29 focused host media tests, WebAssembly compilation, scoped strict
  host lint, TypeScript compilation and formatting. Both route-controlled
  Chromium tests passed after correcting duplicate synthetic catalog digests
  and adding scoped wrapping CSS for bounded long text. They cover desktop and
  mobile layout, pagination, denied-access clearing, exact create confirmation,
  automatic read refresh and association draft preservation. These prove only
  frontend behavior, not persistence, restart or preserved-source planning.
- The focused browser command still exits unsuccessfully because full API
  coverage is absent from this UI-only selection; enforcement remains intact.
  Additional strict WebAssembly lint failed with 751 diagnostics across the UI;
  retained evidence is `target/media-ui-evidence/wasm-clippy.jsonl` and its
  sibling stderr file. Fixed the new list's redundant clone without adding
  suppressions. This checkpoint does not qualify full CI/UI or Sonar coverage.
- Reviewed root, Rust and UI instructions and updated the UI scoped contract.
  No approved design changed. Diagnostics remain bounded and path-free. Before
  release, rollback is reverting the coordinated unshipped UI cutover, not
  restoring incompatible writes or rewriting immutable state.
- Sonar secrets analysis ran on all 16 files in this UI checkpoint without
  findings. This does not replace the previously unavailable connected quality
  analyzer or establish positive published coverage. Exact invocation:

  ```sh
  just --command sonar analyze secrets crates/revaer-api-models/src/media_root_contract/profile_page.rs crates/revaer-ui/src/features/media/profile_list.rs crates/revaer-ui/src/features/media/profile_list/tests.rs crates/revaer-ui/src/features/media/profile_list_view.rs crates/revaer-ui/src/features/media/api.rs crates/revaer-ui/src/features/media/state.rs crates/revaer-ui/src/features/media/view.rs crates/revaer-ui/src/features/media/logic.rs crates/revaer-ui/src/features/media/mod.rs crates/revaer-ui/src/features/media/profile_roots_view.rs crates/revaer-ui/src/features/media/root_catalog_view.rs crates/revaer-ui/src/features/media/association_view.rs crates/revaer-ui/static/style.css .github/instructions/revaer-ui.instructions.md tests/specs/ui/media-profile-list.spec.ts docs/adr/590-profile-version-wire-completion.md
  ```
- Next action: wire the existing root-catalog startup components into the real
  service so logical roots can admit a save, then prove authenticated UI save,
  restart and dry-run source preservation. Association choices still require
  enabled-active-version filtering; a latest version alone is not authority.

## Original Approval Request (Accepted Above)

**Approve the profile wire format and enabled-state behavior below?**
This permits completing the profile editor and its typed API implementation.
It does not approve another database investigation or waive any release gate.

## Recommendation

- Keep ADR 557's collection/item/archive routes, complete conditional writes,
  immutable versions, logical root authority and latest/active-head semantics.
- A complete create/replace body has these required fields:
  `profile_key`, `display_name`, `description`, `enabled`, `dry_run_only`,
  `desired_target_key`, `desired_target_version`, `policy_key`, `policy_version`,
  `output_root_key`, `workspace_root_key`. Optional `backup_root_key` and
  `quarantine_root_key` are omitted when absent; explicit null is rejected.
  No partial update, implicit version, path, discovery mode or retention override
  is accepted. File-selection and processing rules remain in the complete policy
  aggregate owned by ADRs 518/521, not duplicated on the profile.
- Use ADR 521's key, display-name, description and 1 MiB request bounds. Versions
  are positive PostgreSQL integers. Preserve exact typed values; reject unknown
  or duplicate fields. Root-key validation and policy-required optional bindings
  remain exactly ADR 557's contract.
- A response contains the complete submitted fields plus
  `media_profile_public_id`, `latest_version`, optional `active_version`,
  `lifecycle_state`, the ordinal `root_bindings` array specified by ADR 557,
  `created_at` and `updated_at`. Times are UTC RFC 3339 strings. The strong ETag
  is the ADR 557 HTTP header, not a second writable body field. Bindings must
  match the named root keys exactly and contain no filesystem identities.
- `enabled` is distinct from immutable lifecycle. A structurally valid JSON
  create/replace advances latest and active heads even when disabled; operational
  admission additionally requires that active version's `enabled` be true.
  Disabling a profile therefore cannot leave its previously enabled version
  operational. Unmapped YAML drafts retain the previous active head, as already
  required by ADR 557; archive clears it. Association choices show only enabled
  active versions, never merely the newest version.
- New UI drafts start disabled and dry-run-only. Target and policy versions
  require explicit selection. `dry_run_only` can only restrict the selected
  policy: either dry-run setting forbids mutation. Clearing it never overrides
  a dry-run policy. These safety values are explicit in every complete body.
- Preserve edits on 412, load the current complete representation separately,
  and require operator review before another write. Confirmation requires exact
  version/identity/body agreement and the matching strong ETag.

## Alternatives And Consequences

Reusing legacy upsert/PATCH would retain paths, implicit references and partial
mutation. An untyped document would violate ADR 521. The recommended typed
aggregate supports the approved workflow but is a coordinated v0 API/UI/YAML
cutover, not a compatibility alias; readers must understand the enabled flag.

## Task Record

- Motivation: unblock complete profile authoring without inventing public wire
  fields under ADR 559's private-refinement exception.
- Design notes: public spelling and enabled/head behavior above are proposals,
  not previously granted approval. Catalog-only root controls remain under 557.
- Test coverage summary: this proposal has no implementation qualification.
  Require strict decoder/bound tests; create/replace/archive, 412 and disabled
  admission tests; exact round trips; draft-preservation browser tests; real
  persisted-state assertions; and all existing CI/UI/Sonar/package gates.
- Observability updates: retain bounded problem codes and field pointers; no
  paths, raw values, ETags or resource identities in metric labels.
- Risk and rollback: mismatched clients must fail closed, not fall back to
  legacy writes. Before release, revert the coordinated unshipped interface;
  never delete immutable versions or rewrite job references as rollback.
- Dependency rationale: no new dependency or infrastructure is proposed.
- Stale-policy check: reviewed AGENTS.md, Rust/UI scoped instructions, 521,
  557, 559 G1 and 518. Existing legacy models are implementation drift, not
  authority. No accepted constraint or quality criterion is relaxed.
