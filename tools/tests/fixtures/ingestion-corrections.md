# Approved ingestion correction fixtures

`ingestion-corrections.json` was recorded on 2026-09-21 from the existing Ruby
`IngestionApprovedDeltas` transformations in the active media worktree. The table
rows are deliberately small synthetic observations; they are not evidence that
application ingestion has passed. The Python implementation did not generate
the expected final observations.

The D4 and D5 diagnostic SQL statements come from frozen migration 0052. Their
complete `SQL statement "..."` strings reproduce the two approved SHA-256 values
before fixture generation succeeds. Do not update these statements or diagnostic
pins merely to make a comparison pass.

## Source identities

- `scripts/database_rebaseline/ingestion_approved_deltas.rb`: `52a477436708293daa693ff5c99240bdb6f66b40f7254ed773c8570c2783f5f3`
- `crates/revaer-data/migrations/0052_indexer_search_result_ingest_proc.sql`: `621c1421a6cab3731e4e6939c2687942a3bf343ce578f87cbc62cd2deb571930`

## What the fixtures prove

The port must accept exactly the original expected correction and leave both
input observations unchanged. Mutation tests alter individual diagnostics,
outputs and table shapes to ensure the approval does not become a general error
allowance. No test disables the full application's remaining qualification.
The fixture remains authored main-code scanner scope.
