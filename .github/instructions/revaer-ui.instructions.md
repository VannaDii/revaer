---
applyTo:
  - "just/ui.just"
  - "crates/revaer-ui/**"
  - "tests/**"
---

`AGENTS.md` and `rust.instructions.md` apply first. This file specializes UI and E2E work.

- The ADR 591 E2E application uses a complete, sealed single-init disposable
  database and restricted runtime login, not migration bootstrap or admin
  credentials. Require an explicit local test service and `PG_CONTAINER`;
  never reset caller-owned databases. Reject URL query overrides that could
  substitute privileged credentials or another endpoint. Preserve fixture
  ownership metadata across process-state updates and retain it if teardown
  fails. Bootstrap unit tests are not evidence of real service/UI completion.
- ADRs 557/590 profile and association controls use catalog-backed logical keys
  and explicit versions. New drafts remain disabled and dry-run-only. Preserve
  operator edits on rejected or unconfirmed writes; require matching complete
  response identity, version, body and strong ETag before reporting success.
  Mocked browser responses qualify only frontend behavior, never persistence,
  restart continuity or dry-run original preservation.
- Profile readiness exposes complete latest/optional active bodies, active
  logical bindings, separate binding/destructive classes, per-kind root counts
  and bounded association counts. Do not restore path-taking parent payloads or
  interpret reported readiness as permission to execute. Capability qualification
  remains separate and mandatory at admission/execution boundaries.
- The media dashboard reads complete validated profile pages and renders logical
  bindings, exact versions, enablement and the distinct readiness classes. Do
  not restore the retired absolute-path create form, partial profile toggles,
  implicit target/policy versions or path-taking discovery controls. Confirmed
  creates refresh profile reads without clearing unsaved association intent.
  Preserve original discovery/automation coverage while moving it to the
  approved association workflow; retiring an old control is not qualification
  of its replacement or permission to drop the capability.
- Editing loads a complete profile with its matching strong ETag, preserves exact
  logical bindings (including unavailable selections), locks the profile key and
  submits a complete conditional PUT. Confirm the next head, identity, body and
  returned tag before reporting success. A 412 preserves the draft and original
  fence; never automatically rebase or retry. Explicitly confirm discarding a
  current draft before selecting another profile. Authentication changes invalidate
  pending responses and write authority without overwriting an existing draft.
- Schedule cadence authoring uses an independently selected active association
  and its exact version, not manual-trigger permission. Keep quantity/unit empty
  until operator selection; validate approved minute/hour bounds. Conditional
  create requires complete matching readback. Failed/unconfirmed saves preserve
  drafts and require reload; authentication changes invalidate pending responses.
  Edit only from a complete matching strong schedule ETag. A stale edit preserves
  the draft and old fence; reload is explicit, never automatic rebase/retry.
  Cadence persistence/editing is not automation enablement or worker qualification.
- Desired-target changes use that complete fenced profile PUT, never the retired
  target-only PATCH. Prove retired pin/clear requests cannot mutate native heads,
  bodies, ETags or source media; a successful response from a retired writer is
  not evidence of a saved native profile.
- Portable export preserves complete native profile versions and exact logical
  association pins, including referenced bodies older than the latest head.
  Require bounded canonical output without physical paths, host UUIDs or
  attestation evidence. An export pass alone does not qualify validation, atomic
  import, local-path export or the complete configuration round trip.
- YAML validation decodes native complete profiles through the JSON constructor
  and checks explicit versions and logical association pins without writes.
  Reject aliases, recursive input, custom tags, duplicate/non-string keys,
  unknown fields and bounds violations. Quoted or block-scalar punctuation is
  ordinary operator text. Keep the 4 MiB decoded-document limit distinct from
  the bounded JSON escaping envelope; validation never supplies write fences.
  The owned Linux HTTP relay must consume interim informational headers before
  forwarding the final response, including curl's large-body `100 Continue`.
  Prove document boundaries against the real service, not a mocked transport.
- Import write authority is separate from portable YAML. Apply requires an
  explicit `preconditions` array covering every distinct native kind/key with
  create intent or a positive expected head version. Body versions are not
  write fences; never infer overwrite consent from an export or validation pass.
  Preserve operator drafts on rejected intent. Envelope/compiler checks alone
  do not qualify transaction-time head comparison, atomic apply or the UI's
  explicit import confirmation; those require real workflow evidence.
- Manual discovery takes the confirmed active association and bounded relative
  candidates. Saved associations remain available through complete validated,
  paginated collection reads after reload; inactive or binding-unready rows are
  visible but not selectable for manual admission. Pagination, failed reads and
  authentication-context changes clear obsolete selected-association evidence.
  Never accept absolute paths or the retired profile-ID request. Require a
  successful candidate preview and explicit queue confirmation; editing candidates
  clears prior preview/confirmation. Reject incomplete or foreign response rows,
  preserve drafts on failure, and ignore obsolete authentication-context responses.
  Queue acknowledgement is not proof of planning, execution or source preservation.
- Schedule/watcher requests use the same exact association-relative candidate
  contract. Validate bounds and reject retired profile-ID/absolute-path bodies
  before reading an association. Disabled modes must return their bounded
  disabled-mode error without admitting jobs; the existing automatic-activation
  qualification boundary is not permission to fabricate successful runs.
- Schedule/watcher lists contain complete path-free association entries, not
  retired parent paths. Preserve the canonical association collection's bounds,
  cursor validation and continuation; lists report requested modes separately
  from readiness and do not imply automatic activation is qualified.
- The owned Linux operator fixture must prove UI save, real service restart,
  persisted dry-run planning and unchanged source bytes before its destructive
  execution/retry proof. Use separate media and non-overlapping association
  prefixes; do not weaken overlap checks to share a whole-root fixture.
  Service-process restart continuity does not qualify container-remount recovery.
  Changed attestation requires ADR 557's explicit rebinding; never bypass that
  fence or report a process restart as remount/package qualification.
- API fixture consumers share the same owned Linux service wrapper and streaming
  relay as that browser proof. A fixture readiness message is not a workflow
  pass. Do not log its authentication key, reuse another fixture's credentials,
  reset caller data or treat this diagnostic fixture as package qualification.
- Discovery preview must report effective profile-plus-policy dry-run, not merely
  the profile toggle. Clearing a profile restriction cannot override a selected
  policy's output restriction or imply that a queued job will execute.
- Recent-job retry and cancellation controls follow service status transitions,
  require explicit confirmation and disable while in flight. Refresh after every
  response, clear confirmation and never automatically repeat an uncertain write.
  Route-controlled tests are not evidence of real source preservation or recovery.
- Policy output authoring starts dry-run with replacement disabled and all
  quarantine/preservation switches enabled. Submit the complete output settings,
  confirm exact returned settings and version before reporting success, and
  preserve the draft on failed or unconfirmed writes. Portable policies preserve
  those settings; imported profiles remain forced to dry-run independently.

# First-Party Vs Vendor Paths

- First-party authored UI quality targets are:
  - `crates/revaer-ui/src/**`
  - `crates/revaer-ui/i18n/**`
  - `crates/revaer-ui/tools/asset_sync/src/**`
  - `tests/**` excluding generated or installed dependencies
- Generated or vendored paths are not first-party authored UI code:
  - `crates/revaer-ui/ui_vendor/**`
  - `crates/revaer-ui/static/nexus/**`
  - `crates/revaer-ui/dist/**`
  - `crates/revaer-ui/dist-serve/**`
  - `crates/revaer-ui/target/**`
  - `tests/node_modules/**`
  - `tests/logs/**`
  - `tests/test-results/**`
- Do not hand-edit vendored or generated assets unless the task is explicitly about vendor ingestion, asset synchronization, or generated output shape.

# UI Architecture

- `app/*` is the only layer that touches browser globals, storage, router providers, or `EventSource`.
- `core/*` stays DOM-free and host-testable.
- `services/*` is transport-only. Convert DTOs into feature state before they reach UI views.
- `features/*` owns vertical slices. Features do not reach into each other directly.
- `components/*` hosts shared UI building blocks only. No persistence, API calls, or SSE side effects inside shared components.
- `models.rs` contains transport DTOs only. UI-only fields live in feature state.

# UI And E2E Maintenance

- ADR 586/588 permits an explicitly selected test-only compliance loader for
  canonical E2E. `just ui-e2e-app-build` builds the app library test executable;
  setup must select its exact current-source Cargo JSON artifact and launch
  only `bootstrap::runtime_tests::e2e_serving_entry`. Preserve the real shared
  preflight, serving runtime, disposable database, setup/auth configuration,
  complete Playwright suites, route assertions and coverage gates. Never use
  the limited injected-success smoke test's Active/NoAuth configuration or
  count that smoke test as the full UI gate. Production entrypoints must not
  select fixtures or gain a bypass. This test path is not C1-D package proof.
- E2E setup must refuse occupied ports without searching for or terminating
  other development processes. Its Trunk server disables automatic browser reload
  so coverage artifacts and screenshots cannot erase in-progress form state.
  Retain compilation and error reporting; this is not a production-server setting.
  Record each owned database/process before the
  next fallible setup step so existing teardown can clean failed starts.
  Keep artifact-selection and non-destructive port regressions in
  `just ui-e2e-bootstrap-test`, including its strict bootstrap typecheck;
  all Node execution uses the existing wrapper. `just ui-e2e-app-test` must
  exercise the exact serving entry and production preflight regressions against
  an explicitly supplied disposable database without changing auth/setup state.
- Keep selectors and test affordances stable. Update E2E fixtures deliberately when UI structure changes.
- Root-readiness presentation consumes the validated ADR 557 response and keeps
  binding and destructive slot counts separate. Clear old evidence on refresh
  or access failure; ignore completions from an obsolete API context. Never
  display raw transport diagnostics or use readiness counts as write authority.
  Route-controlled presentation tests are not root-attestation or service E2E proof.
- E2E setup must allowlist exactly the resolved private fixture root in the
  existing filesystem policy, preserving its other fields and the fetched
  snapshot. Cover absolute/relative roots under both authentication modes;
  never bypass production path validation to make torrent authoring succeed.
- API E2E media tests must create real source files beneath private per-test temporary profile roots and remove the complete temporary tree in `afterEach`, including when an assertion fails. Synthetic paths may be used only for routes whose contract explicitly rejects or never reads the filesystem.
- Legacy path-taking profile fixtures must follow ADR 419: create in dry-run with automation disabled and no cadence; metadata-only updates must not supply scheduling intervals. Retain positive persistence checks and explicit rejection tests for unverified automation or interval changes, including unchanged persisted state. Missing included automation capabilities remain failures, not permission to fabricate root identity, remove workflow assertions, or count injected-host checks as package/full E2E proof.
- API E2E route coverage must reject `405` for every supported media operation and must separately require exact `405` responses for worker-owned `POST /v1/media/jobs` and `POST /v1/media/jobs/{media_job_public_id}/phases`; do not restore those retired writes or remove them from end-to-end coverage.
- Treat generated API clients and synchronized assets as generated artifacts; regenerate them intentionally and keep authored wrappers separate.
- Run every Node command through `scripts/with-node.sh`. The wrapper must select and verify the exact `.nvmrc` version so an operator's NVM-managed Node remains active rather than being replaced by a login shell.
- `tests/playwright.config.ts` must resolve reporter artifacts from the exact `E2E_ENV_DIR` supplied by `just ui-e2e`, with the configuration directory as the direct-invocation fallback, so both source and coverage-compiled runs write the HTML report to `tests/playwright-report`.
- Every PR UI E2E shard must upload nonempty API and UI coverage with `if-no-files-found: error`. The aggregate must download all three exact named shard artifacts and run `just ui-e2e-shard-coverage` before evaluating combined route coverage; a missing shard or record is a failed gate.
- `asset_sync` must fail closed unless every required runtime SVG exists, runtime text is UTF-8, SVG files have a complete namespaced root envelope, and `crates/revaer-ui/static` contains no raster-extension assets.
- `static/revaer-logo.svg` and `static/icons/app-icon.svg` must preserve the approved purple stylized-R composition and the `revaer-purple-gradient` and `revaer-r-silhouette` identifiers; do not substitute a wordmark, palette, or symbol during asset synchronization.
- Keep emitted icon, logo, dashboard, and DataTables references rooted under `/static`; verify their targets in a Trunk release build rather than inferring paths from source layout.
- `just check-assets` must compare `crates/revaer-ui/static/nexus` from the repository root after regeneration and fail unless repository-root `revaer-logo.svg` is byte-identical to `crates/revaer-ui/static/revaer-logo.svg`.
- Keep legacy vendor-reference canonicalization in `asset_sync`, validate the UTF-8 served image set there, and make `just check-assets` compare the complete repository-root `crates/revaer-ui/static/nexus/**` output.
- CI E2E should use an explicit browser channel such as `E2E_BROWSER_CHANNEL=chrome` when the runner already provides that browser, so shards install Playwright dependencies without downloading redundant browser bundles. Keep CI video capture off for that path unless the Playwright ffmpeg bundle is installed.
- When UI structure, selectors, or synced assets change, update the relevant docs, tests, and instructions in the same change.
