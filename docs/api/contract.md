# HTTP API contract

[`openapi.json`](openapi.json) describes the HTTP contract. The API embeds this
document and serves it at `/docs/openapi.json`. The existing Rust export command
serializes the embedded document; it does not infer schemas from Rust structs.

## Maintaining the document

Review the router and handler status codes in `crates/revaer-api/src/http/`, then
the Serde representation of request and response structs in
`crates/revaer-api-models/src/lib.rs`. Reflect omitted optional fields, flattened
structs, list wrappers, path/query parameters, and authentication errors exactly.
Keep existing paths and schemas intact when adding missing contract details.

The Python E2E client validates each observed status, content type and JSON body
against this document. Its scenarios also make explicit behavior assertions.
Run `rv ui-e2e` for anonymous, authenticated and browser phases, followed by
`rv ui-e2e-coverage` for operation coverage. Unknown operations or statuses fail
with their route identity; they are not accepted through a generic fallback.

## Corrections found during the Python migration

The old TypeScript client's types did not validate runtime response bodies. The
Python port exposed missing indexer inventory, Cardigann, notification, RSS and
connectivity operations, plus the existing tag deletion route. Their schemas
now follow the existing handlers and models. Setup completion documents its
already-configured `409` response. Torrent detail composes `TorrentSummary` at
the object root, matching `#[serde(flatten)]`, instead of requiring a nested
`summary` property that the API does not send.

These are corrections to the description of existing behavior. No handler,
database operation or authentication behavior changes with these contract edits.
