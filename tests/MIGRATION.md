# Scenario migration map

This map records the foundation at `54641c63`. The original TypeScript scenarios
remain available for comparison until the complete tooling/workflow migration is
accepted. Python combines dependent serial steps into complete scenario functions;
scenario counts therefore differ from the old runner's test count.

## API scenarios

All paths below are relative to `specs/api/`. Every Python scenario runs once with
anonymous access and once with API-key authentication.

| Original file | Python replacement | Preserved behavior |
| --- | --- | --- |
| `public.spec.ts` | `test_public.py` | Health, well-known configuration, metrics, OpenAPI export. |
| `auth.spec.ts`, `setup.spec.ts` | `test_auth.py` | Refresh behavior, protected endpoint access, rejection of setup after activation. |
| `config.spec.ts`, `filesystem.spec.ts` | `test_config.py` | Dashboard, config reads/patches, admin settings, allowed filesystem browsing. |
| `events.spec.ts` | `test_events.py` | Log stream headers and triggered event streams. |
| `torrents.spec.ts` | `test_torrents.py` | Create/list/detail, categories/tags, authoring, selection/options/actions, tracker/webseed validation, peers and removal in one lifecycle. |
| `indexers-definitions.spec.ts`, `indexers-definitions-import.spec.ts` | `test_indexer_catalog.py` | Catalog reads and Cardigann field/option import. |
| `indexers-instances.spec.ts` | `test_indexer_catalog.py` | Missing-instance management, connectivity, Cloudflare, RSS, reputation and health-event boundaries. |
| `indexers-category-mappings.spec.ts` | `test_indexer_categories.py` | Global and per-indexer category mappings, Torznab overrides, unknown-instance errors. |
| `indexers-routing-policies.spec.ts`, `indexers-rate-limits.spec.ts` | `test_indexer_connectivity.py` | Routing policy credentials and rate limits. |
| `indexers-health-notifications.spec.ts` | `test_indexer_connectivity.py` | Notification hook create/list/update/delete. |
| `indexers-policies.spec.ts` | `test_indexer_policies.py` | Policy sets/rules, enable/disable, ordering, invalid expiry. |
| `indexers-search-profiles.spec.ts` | `test_indexer_profiles.py` | Profile defaults, domains/tags, policy/indexer associations and missing references. |
| `indexers-import-jobs.spec.ts`, `indexers-final-acceptance.spec.ts`, `indexers-coexistence-rollback.spec.ts` | `test_indexer_imports.py` | Import source validation, dry runs, state/results, acceptance and coexistence/rollback boundaries. |
| `indexers-search-requests.spec.ts` | `test_indexer_search.py` | Search types, creation, pages and cancellation boundaries. |
| `indexers-secrets.spec.ts`, `indexers-tags.spec.ts` | `test_indexer_secrets_tags.py` | Secret rotation/revocation and tag deletion by body/path. |
| `indexers-torznab-instances.spec.ts`, `indexers-migration-parity.spec.ts` | `test_indexer_torznab.py` | Capabilities/search, paging, invalid query combinations, authentication, disabled instances, download and management boundaries, migration behavior. |

Category scenarios create their own catalog/profile. Torznab creation must return
its real identifier and key before subsequent assertions can run. These replace
collection-order dependencies and conditional assertion skipping in the old suite.
Response schema validation also exposed missing operation/status descriptions and
the flattened torrent-detail shape in the committed OpenAPI document; the contract
was corrected against the existing handlers and Serde models.

## Browser scenarios

All paths below are relative to `specs/ui/`.

| Original file | Python replacement | Preserved behavior |
| --- | --- | --- |
| `dashboard.spec.ts`, `health.spec.ts` | `test_navigation.py` | Overview and health routes. |
| `navigation.spec.ts`, `settings.spec.ts` | `test_navigation.py` | Sidebar destinations, icon controls, layout bounds and settings tabs. |
| `torrents.spec.ts`, `routes.spec.ts` | `test_torrents.py` | List controls, add/create modals, menu stacking, detail route and not-found page. |
| `logs.spec.ts` | `test_logs.py` | Empty terminal height, five log levels, filtering and search. |
| `indexers.spec.ts` | `test_indexers.py` | Indexer panels and controls. |

Native storage state replaces JavaScript storage interception. Playwright trial
clicks replace `elementFromPoint` evaluation for menu hit-testing. The detail-route
locator identifies the loading indicator specifically, because error toasts also
have a `status` role. Unexpected local server errors fail the browser scenario.

## Acceptance evidence

On 2026-09-15, foundation `rv ui-e2e` and `rv ui-e2e-coverage` passed with Linux
libtorrent 2.0.10: 70 API executions and 13 Chromium scenarios. The latest run
includes the unexpected-server-response check. Its source fingerprint and logs
are recorded in `artifacts/foundation-attempts/248eb7f6b7eb72f2/`.

Real fixture tests cover Chromium, Firefox and WebKit, retry artifacts, closed-page
videos, setup failure, timeout, parallel workers, exact shard assignment, and the
difference between an expected empty shard and an empty suite. Application runs
on Firefox/WebKit and the media stack remain separate verification work.

## Single-init media integration

Media scenarios are additive and selected only in the single-init feature phase.
The active media worktree is left unchanged; validation runs in a disposable
combined checkout. This table describes port status, not full gate completion.

| Media TypeScript source | Python replacement | Preserved behavior/status |
| --- | --- | --- |
| API `media-root-readiness.spec.ts` | `specs/media/api/test_root_readiness.py` | Missing persisted catalog, bounded input errors, five path-free readiness kinds. Passed under both authentication modes. |
| API `media-profile-create.spec.ts` | `specs/media/api/test_profile_creation.py` | Eleven persistence, dry-run, source preservation, admission and rejection scenarios. Passed under both authentication modes. |
| API `media.spec.ts` | `specs/media/api/test_routes.py` | Three catalog-update/router/retired-write scenarios plus five independent `test_lifecycle.py` scenarios preserve the combined profile/job/import lifecycle. The positive automation assertion conflicts with current admission behavior and remains a visible failure. |
| UI `media-root-readiness.spec.ts` | `specs/media/ui/test_root_readiness.py` | Three refresh, error-redaction, readiness separation and desktop/mobile presentation scenarios. |
| UI `media-association.spec.ts` | `specs/media/ui/test_association.py` | Five explicit-scope, precondition, conflict, pagination, kind-filtering and draft-retention scenarios. |
| UI `media.spec.ts` | `specs/media/ui/test_management.py` | Management controls and real target/policy writes. Passed on Chromium, Firefox and WebKit. |

All nine media UI scenarios passed with the thirteen foundation scenarios on
Chromium, Firefox and WebKit. WebKit initially exposed automatic root selection
on catalog refresh; the unchanged assertion now passes with an integration-only
DOM reconciliation fix. Both API authentication phases passed all 52 selected
cases. Logs, reports, cleanup receipts and source fingerprints are retained in
`artifacts/media-five-phase-acceptance/`. The subsequently ported combined API lifecycle exposes a pre-existing automation
contract conflict described below, so this is not final migration acceptance.
Mocked catalog/readiness routes prove UI behavior only, not root attestation or
persistence.

### Lifecycle acceptance conflict

The complete API port now selects 57 scenarios per authentication phase. Both
anonymous and API-key runs passed 56 and failed the same retained original
requirement: enable scheduling with an interval on a legacy path-based profile.
The application returns 400; the original scenario requires 200. The newer
profile-creation suite explicitly requires the rejection code
`media_profile_filesystem_identity_required` for unverified automation.

The target/policy validation, retention settings, capability/compliance, and
YAML import/export lifecycle scenarios passed in both modes. The automation
scenario remains failing, with later job assertions retained behind its required
admission step. No skip, expected-failure marker, fabricated root identity, or
weakened status assertion was added. Positive automation requires the media
implementation's verified-root path; the conflicting old positive expectation
cannot be declared covered by the rejection tests. Evidence and cleanup receipts
are under `artifacts/media-lifecycle-anonymous/` and
`artifacts/media-lifecycle-authenticated/`.
