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
