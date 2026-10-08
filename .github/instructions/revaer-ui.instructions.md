---
applyTo:
  - "crates/revaer-ui/**"
  - "tests/**"
---

`AGENTS.md` and `rust.instructions.md` apply first. This file specializes UI and E2E work.

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

- Keep selectors and test affordances stable. Update E2E fixtures deliberately when UI structure changes.
- Treat generated API clients and synchronized assets as generated artifacts; regenerate them intentionally and keep authored wrappers separate.
- `asset_sync` must fail closed unless every required runtime SVG exists, runtime text is UTF-8, SVG files have a complete namespaced root envelope, and `crates/revaer-ui/static` contains no raster-extension assets.
- `static/revaer-logo.svg` and `static/icons/app-icon.svg` must preserve the approved purple stylized-R composition and the `revaer-purple-gradient` and `revaer-r-silhouette` identifiers; do not substitute a wordmark, palette, or symbol during asset synchronization.
- Keep emitted icon, logo, dashboard, and DataTables references rooted under `/static`; verify their targets in a Trunk release build rather than inferring paths from source layout.
- `rv check-assets` must compare `crates/revaer-ui/static/nexus` from the repository root after regeneration.
- Correct retained Nexus stylesheet fragments in the canonical vendored
  `html/assets/app.css`, then regenerate through `rv sync-assets`. Preserve
  selector specificity, shorthand precedence and conditional overrides when
  consolidating rules; do not patch the served stylesheet independently.
- Keep legacy vendor-reference canonicalization in `asset_sync`, validate the UTF-8 served image set there, and make `rv check-assets` compare the complete repository-root `crates/revaer-ui/static/nexus/**` output.
- CI E2E should use an explicit browser channel such as `E2E_BROWSER_CHANNEL=chrome` when the runner already provides that browser, so shards install Playwright dependencies without downloading redundant browser bundles. Keep CI video capture off for that path unless the Playwright ffmpeg bundle is installed.
- When UI structure, selectors, or synced assets change, update the relevant docs, tests, and instructions in the same change.
- The accepted Python migration uses the selected checkout's OpenAPI document
  directly for response validation. Keep typed request inputs separate from raw
  transport data and preserve every existing API/UI assertion when porting tests.
- Keep anonymous API, authenticated API, and browser phases ordered. The default
  API executable is configured in `tests/e2e.toml`; media's library-test serving
  entry must be selected explicitly during integration and resolved from Cargo's
  actual artifact output. Never guess an executable in another target directory.
- Use native Playwright storage state and locator operations where available.
  Record failures and retries without replacing browser storage methods with
  authored JavaScript. Do not claim browser parity from runner fixture tests.
- OpenAPI response validation must fail with an operation-specific diagnostic
  for missing paths, methods or statuses. Correct the contract against existing
  handlers and Serde models when drift is found; do not bypass validation to
  reproduce the old generated client's unchecked runtime behavior.
- Browser scenarios must report local HTTP 500 responses as failures. Keep
  credentials out of diagnostics. Shard aggregation must prove completed phases
  and exact scenario assignments; route files alone do not prove successful tests.

## Single-init E2E ownership

- E2E connection defaults follow the environment-supplied credential policy in `devops.instructions.md`; the tracked defaults file is not a credential store.

- In the approved `feature-development` phase, Python E2E uses the complete
  initializer and sealed restricted runtime role. Require explicit test-service
  selection and verify its exact loopback binding before provisioning.
- Keep generated credentials private, bind init/drop to the resolved container
  ID, refuse malformed phase configuration, and stop every owned service before
  dropping the test database. Lifecycle/ordering fixtures are not full media
  application or browser acceptance.

- The media phase adds Python media API scenarios to both authentication phases;
  it must not replace or omit foundation scenarios. Keep the original library
  test-serving entry as the media default and preserve explicit runner overrides.
  Do not bypass production compliance startup checks to run a test suite.

- In the single-init feature phase, the Python E2E adapter adds `tests/specs/media/ui` alongside the foundation UI suite. Route-controlled media presentation tests must remain explicitly distinguished from filesystem or persistence evidence.

- `rv ui-e2e-app-test` preserves the media launch-guard and compliance library
  filters with one test thread, then the `bootstrap` integration binary, under
  default features. Require an explicit disposable test endpoint and positive
  passing execution in every group; do not count empty filter matches as coverage.

# Accepted media contracts

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

## Continuation fixture reconciliation

- Single-init media E2E retains missing-catalog cases as separate anonymous and
  authenticated phases. Each authentication reset runs on an owned service with
  an absent catalog source; join it before starting the scenario service. Native
  positive cases require that restart to attest the configured real Linux catalog
  after reset. Preserve all five default phase summaries, selections, raw coverage
  and separate setup/scenario logs. Never reseed attestation or add runtime catalog
  reloads to compensate for factory reset clearing the catalog tables.

- ADR 595 retains the old media E2E failures while porting to complete immutable-profile requests and logical catalog bindings. Preserve source-integrity, conditional-write, unavailable-root, association, schedule/watcher and cleanup assertions; adapting payloads does not authorize accepting automation before its approved runtime is integrated.
- Rejection of retired path/automation profile bodies is a request-schema guard:
  require `media_configuration_invalid`, no persisted profile and unchanged source
  bytes. It does not prove disabled association admission or native root readiness;
  positive persistence/discovery scenarios require the real active Linux catalog.
- Controlled root selects must restore the exact draft after catalog/render changes without dropping existing edit/authentication fences.

- Ordinary association creation preserves explicitly selected schedule/watcher
  modes after the bounded native loop integration. Keep normal positive activation,
  unchanged-source and disabled-mode assertions; synthetic mode edits never
  substitute for operator configuration through the API and UI.
