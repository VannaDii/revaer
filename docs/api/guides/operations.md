# Operational Setup & Security

## API key lifecycle

1. Issue keys via settings changesets (`SettingsChangeset::api_keys`).

```json
{
  "api_keys": [
    {
      "op": "upsert",
      "key_id": "automation",
      "label": "ci",
      "secret": "<generated>",
      "rate_limit": { "burst": 60, "per_seconds": 60 },
      "enabled": true
    }
  ]
}
```

2. Secrets are hashed at rest. Rotate keys by supplying a new `secret` value.
3. Disable keys with `{ "op": "upsert", "key_id": "automation", "enabled": false }`.
4. Remove keys with `{ "op": "delete", "key_id": "automation" }`.

All authenticated requests must include `x-revaer-api-key: key_id:secret`.

## Live manual search

Create an indexer instance using the built-in `torznab` definition. Set its `base_url` through the existing instance field API, create an encrypted `api_key` secret and bind that secret to the instance's `api_key` field. Enable the instance and interactive search. Credential encryption uses the application's existing configured database session key; never put an API key in an example URL or log it.

Open `/search`, enter a free-text query and select **Search**. Use **Refresh pages** to retrieve sealed results. The production dispatcher claims queued work, sends the authenticated Torznab request and ingests results through the normal policy, normalization and canonicalization procedures. Empty responses seal an empty page; failed and deferred sources appear in explainability. Duplicate releases retain one canonical page position. RSS polling and automatic downloading are separate workflows.

Credential-bearing upstream links are retained for server-side acquisition and omitted from manual-search page responses. Safe details links and release metadata remain available.

## Public Cardigann HTML search

Import a public Cardigann YAML definition through **Import Cardigann definition** on `/indexers` or `POST /v1/indexers/definitions/import/cardigann`. Create an instance from that definition, enable interactive search and include it in the chosen search profile. Public HTML instances need no upstream API key. Ordinary downstream Torznab `t=search` requests use the same adapter; structured movie/TV modes fail actionably rather than becoming generic searches.

The initial subset follows [Cardigann's definition structure](https://github.com/Servarr/Wiki/blob/master/prowlarr/cardigann-yml-definition.md): `type: public`, UTF-8 HTML, empty or absent settings, public credential-free `links`, `caps.modes.search: [q]`, one static GET `search.paths` entry, literal `inputs` and `{{ .Keywords }}` substitution. Path inputs override inherited search inputs; `inheritinputs: false` replaces them. Inputs are encoded as query parameters and empty values are omitted. CSS `rows.selector` and field `selector` plus optional `attribute` extract title, details/download HTTP links or magnet, category, size, seeders, leechers and date. Size is a nonnegative decimal byte count, counts are nonnegative integers, and supplied dates must be RFC3339. Relative links resolve against the requested page. Category mappings use `caps.categorymappings` entries with upstream `id` and a supported standard `cat` name; unknown categories fail rather than inventing metadata.

Title, byte size and at least one details/download/magnet field are required. Optional fields may be omitted from the definition; when configured, their values must be present and valid in every selected row. A valid page with no selected rows is an empty success. Invalid rows fail the attempt through existing error policy. Responses retain the shared two-MiB limit and deadline; extraction permits at most 50,000 DOM nodes and 1,000 releases. Existing profile selection, rate budgets, proxy routing, cancellation, retry policy, ingestion, canonical deduplication and sealed paging still apply. Credential-bearing result links remain private acquisition targets and are omitted from search responses.

Login, private/configurable indexers, POST, JSON/XML extraction, template control flow, transformation filters, pagination and separate RSS paths are unsupported. Imports reject unsupported executable requirements before making any upstream request. Reimport older metadata-only definitions to install executable rules; their existing metadata alone cannot execute requests. Rules and definition identity are published in one transaction and survive application restart. This subset does not imply compatibility with the full Cardigann catalog or completion of Prowlarr migration.

## Scheduled Torznab RSS

Configure the upstream endpoint and encrypted API key as for manual search. Enable the indexer and its **RSS enabled** setting, then enable its subscription using **Update RSS subscription** on `/indexers` or `PUT /v1/indexers/instances/{indexer_instance_public_id}/rss`. The interval accepts 300–86400 seconds. The production `rss_poll` schedule discovers due subscriptions independently of search-profile selection and sends `t=search` with an empty query, preserving endpoint filters.

Use **Fetch RSS status** and **Fetch recent RSS items** to inspect the last successful poll, next scheduled time, error/backoff and deduplicated identifiers in the existing Activity log. Refresh remains explicit. RSS and Atom identifiers do not require full search-release metadata; repeated items preserve their original first-seen time. RSS neither populates canonical search results nor adds torrents or downloads.

Successful polls follow the configured interval plus existing jitter. Network failures, timeouts, HTTP 429/5xx and budget denials retain retry backoff; authentication and malformed-feed failures disable the subscription with a reason. Correct the configuration before re-enabling it. Configured proxy failures never fall back to direct access. Shutdown settles owned work; bounded persisted leases let a later process recover interrupted polling, and stale completions cannot overwrite newer subscription settings.

### Scheduled public Cardigann discovery

The same RSS controls also support imported public HTML definitions whose existing static GET path returns recent releases when keywords are empty. Enable the instance, **RSS enabled**, and its subscription using **Update RSS subscription**. Literal inputs such as `order: newest` remain present; empty keyword inputs are omitted. No alternate RSS path, Torznab API parameters, login or new template syntax is added. Search-profile exclusion does not affect RSS eligibility.

Use **Fetch RSS status** and **Fetch recent RSS items** on `/indexers` to inspect actual scheduled discovery. Valid magnets produce normalized identifier hashes; safe details links provide fallback identifiers under the existing length and credential checks. Download targets, credential-bearing links and raw magnets are not persisted as RSS identifiers. Valid rows without an eligible identifier are skipped; empty, repeated and identifier-free listings can still succeed. Repeated rows and polls preserve original first-seen times. Polling records RSS outbound activity without creating search requests, canonical search results or downloads.

Cardigann uses the same routing, response limits, timeouts, budgets, backoff and fenced completion as Torznab RSS. Missing executable rules produce reimport guidance in the application log before HTTP; reimport the definition and re-enable its subscription. Existing HTML field validation remains required, including for recent listings. Rules are read as one persisted snapshot authorized by the current RSS claim and remain available after application restart.

## Downstream Torznab search

Bind a Revaer Torznab instance to a search profile that permits the configured upstreams and their media domains. Downstream clients call `/torznab/{torznab_instance_public_id}/api` with that instance's generated `apikey`, using `t=search`, `tvsearch` or `movie`. This downstream key is distinct from each encrypted upstream credential. The bound profile controls indexer eligibility, categories and policy snapshots.

The endpoint uses the same production dispatcher as manual search. It waits up to five seconds for the requested sealed result window or terminal search state, returning sooner when ready. At the deadline it returns HTTP 200 with available sealed results; unfinished, retrying or deferred work continues normally. An empty response can therefore mean that results were not ready by the deadline. Each later request creates a new search; there is no cross-request cache or shared snapshot guarantee.

External `offset` skips flattened append-order results and `limit` applies afterward, including windows crossing internal pages. The profile controls internal page size. Feed details/download links remain scoped to the authenticated Revaer instance and preserve the existing authorized acquisition redirect; upstream credentials and private acquisition targets are never embedded in the feed.

## Rate limiting

- Token bucket parameters are `burst` (max outstanding requests) and `per_seconds` (refill interval).
- Exceeded limits return `429` Problem+JSON responses plus `x-ratelimit-*` headers.
- Metrics include `api_rate_limit_throttled_total` and `config_guardrail_violations_total`.
- Guardrail violations clear automatically after applying a valid rate limit.

## Secrets and logging

- CLI never prints secrets; API keys must be supplied as `key_id:secret` header values.
- Logs and traces redact secrets; rate-limit warnings include only `key_id`.
- Setup completion requires `x-revaer-setup-token` and returns the generated API key once.

## Docker runtime

- Container runs as a non-root `revaer` user with `/data` and `/config` volumes.
- Healthcheck targets `/health/full`.
- For production, run with a read-only root filesystem, persist `/data` and `/config`, and provide a writable `tmpfs` for `/tmp` if your runtime enforces `--read-only`.
- Drop ambient Linux capabilities unless your orchestrator adds them by default; normal operation does not require elevated capabilities.
- Published images are scanned in CI, built with SBOM/provenance attestations, and signed with Cosign during the image publish workflow.

## SSE and resume

- SSE endpoints accept `Last-Event-ID` for replay. Prefer `/v1/torrents/events` and fall back to `/v1/events/stream`.
- CLI `revaer tail --resume-file <path>` persists event IDs across reconnects.

## Rotation playbook

1. Create a new key with overlapping scope and rate limit.
2. Update automation to use the new secret; confirm via CLI telemetry.
3. Disable the old key and watch `http_requests_total` for drain.
4. Delete the old key once stale traffic drops to zero.
