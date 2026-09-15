# D3 helper source dispositions

Status: In progress; bounded static mapping, **not D3 acceptance**, approval, or a new execution result.
Inspected source: detached `1694ffb6c67675ff68b7469716ef79ca99f049bd`, 2026-09-14.
Scope: the ten names in [`INGESTION_HELPERS`](../../../scripts/database_rebaseline/ingestion_proof.rb#L61) other than `search_result_ingest` and `search_result_ingest_v1`.
[ADR 569 D3](../569-init-privilege-and-timeout-resolution.md#d3-function-local-variable-resolution) and the [closure map](569-d3-closure-map.md) govern; caller/wrapper closure and integration remain parent-owned.

## Source and witness boundary

- Read-only extraction compared all ten complete `AS $$...$$` bodies against their latest numbered migration definitions, then against `init-candidate.sql` and `final-observed-schema.sql` under `/private/tmp/revaer-approved-integration/target/database-rebaseline/`: all ten match byte-for-byte.
- The observed-schema file is retained generated SQL, not a fresh live catalog assertion. Source links below identify the exact frozen definitions; migration 0066, not 0052, supplies the latest title normalizer.
- **H**: [helper-first SQL](../../../scripts/tests/database-ingestion-helper-first.sql), consumed by [`ingestion_parse` / `ingestion_helper_expectations`](../../../scripts/database_rebaseline/ingestion_proof.rb#L342), establishes the explicit pure-helper answers before ingestion under caller `error`.
- **P**: [`policy_cases`, `policy_null_cases`, `policy_helpers_sql`, `policy_verify!`, `policy_context?`](../../../scripts/database_rebaseline/ingestion_policy.rb) bind populated inputs, results, mutations, errors and cold/helper-first backend/settings context.
- **N**: [`NativePolicyPhases#validate!`](../../../scripts/database_rebaseline/native_policy_phases.rb) binds five named policy scenarios in cold/helper-first reference/final contexts; [`NativePolicyCastScope#validate!`](../../../scripts/database_rebaseline/native_policy_cast_scope.rb) handles reference SQL inlining separately.
- **A**: [`dependency_native_answers!` / `dependency_validate_native_answers!`](../../../scripts/database_rebaseline/ingestion_dependencies.rb#L680) bind accented normalization, both digest overloads and whitespace-only derivation before/after commit on one backend.
- Existing execution qualification is retained in closure-map rows 1, 2, 4, 5, 7 and 11. This note inspected method/scenario source, not raw traces or report archives; it neither reruns nor requalifies that evidence.

## Ten finite dispositions

### 1. `normalize_title_v1`
[0066:3](../../../crates/revaer-data/migrations/0066_indexer_proc_fixes.sql#L3); generated final-observed schema:16333.
NULL returns before `unaccent`; otherwise lower/unaccent, fixed regex replacements and the fixed token loop run before trim/return.
There is no relation-bearing query: `value`, `token`, `tokens`, `title_raw` cannot collide with query columns; constructed regex patterns are data, not dynamic SQL.
H covers NULL and token removal; A covers accented text cold/committed-warm; [`attributes_cases`](../../../scripts/database_rebaseline/ingestion_attributes.rb#L53) retains suffix/token outcomes. No additional helper-local compilation branch identified.

### 2. `normalize_magnet_uri_v1`
[0052:64](../../../crates/revaer-data/migrations/0052_indexer_search_result_ingest_proc.sql#L64); generated final-observed schema:16256.
NULL/blank, non-magnet, absent `?`, empty query, filtered-empty aggregation, and populated aggregation are distinct returns; CASE distinguishes bare keys from `key=value`.
The sole SELECT reads `unnest(params)` as `param`, producing `key_part`/`value_part`; none is a PL/pgSQL parameter/local. `params` and INTO `normalized_parts` are variables with no competing column names.
H explicitly covers every listed return and both CASE arms; [`wrapper_identity_specs`](../../../scripts/database_rebaseline/ingestion_identity.rb) provides `non-magnet-uri`, `magnet-no-query`, `magnet-empty-query`, `magnet-empty-keys`, `magnet-bare-key` ingestion inputs.
[`NativeWarmHelperProof`](../../../scripts/database_rebaseline/native_warm_helper_proof.rb) adds the exact `magnet-uri` cold/committed-warm context, not every URI's native trace. The other URI values introduce no new SQL name-resolution context.

### 3. `derive_magnet_hash_v1`
[0052:126](../../../crates/revaer-data/migrations/0052_indexer_search_result_ingest_proc.sql#L126); generated final-observed schema:2604.
Non-NULL v2 wins before v1; either decodes hex and calls bytea `digest`. Only absence of both reaches the URI NULL guard, nested `normalize_magnet_uri_v1`, its NULL-result guard, then text `digest`.
H supplies v2-over-v1, v1, all-NULL and URI paths; A supplies the otherwise omitted whitespace URI -> nested NULL -> no digest path and both digest-overload answers. Identity scenarios bind ingestion entry relevance.
No SELECT/FROM or identifier-bearing SQL is constructed. Decode/native failures propagate without exception handling; arbitrary direct-helper malformed hex is not a new variable/column compilation context. Parent owns caller validation/reachability, not a newly asserted success here.

### 4. `compute_title_size_hash_v1`
[0052:162](../../../crates/revaer-data/migrations/0052_indexer_search_result_ingest_proc.sql#L162); generated final-observed schema:2423.
Either NULL argument skips text `digest`; otherwise concatenate title, literal separator and size text, digest/encode/lower. No relation scope or authored nested helper exists.
H covers each NULL operand and a concrete hash; `new-title-size` in [`ingestion_cases`](../../../scripts/database_rebaseline/ingestion_proof.rb#L200) supplies cold/helper-first ingestion, and `NativeWarmHelperProof` supplies `title-size` cold/committed-warm. No uncovered helper-local context.

### 5. `policy_text_match_v1`
[0052:184](../../../crates/revaer-data/migrations/0052_indexer_search_result_ingest_proc.sql#L184); generated final-observed schema:18591.
Candidate NULL skips all matching; regex has NULL-operand and sensitive/insensitive arms; eq/contains/prefix/suffix have operand guards; in-set has a NULL-set guard and one EXISTS; unsupported/NULL operator falls through FALSE.
Only EXISTS has a relation scope: `value_set_id`/`value_text` are table columns; all parameters end `_input`, and `candidate_norm` is absent from the exact value-set table. Regex input is not executable SQL.
H and P cover the operators/guards, populated matches/nonmatches, NULL sets and invalid regex; P names `null-{regex,eq}-operand-*` and `release-regex-error-*`; N binds nested regex error dispatch. Remaining fallthrough merely returns FALSE with no extra query/call; no D3 ambiguity gap identified.

### 6. `policy_uuid_match_v1`
[0052:243](../../../crates/revaer-data/migrations/0052_indexer_search_result_ingest_proc.sql#L243); generated final-observed schema:18659.
NULL candidate -> FALSE; eq returns the comparison (including SQL NULL); in-set guards NULL set then EXISTS; other operators -> FALSE.
EXISTS uses column-only `value_set_id`/`value_uuid` against `_input` parameters; no locals or dynamic calls. H covers NULL/eq/empty-set/unsupported; P `populated-all-fields`, `populated-nonmatches`, `null-set-ids`, `operator-eq` and helper answers cover populated branches; N includes this helper in `populated-all-fields`.
NULL comparison operands change truth value, not query binding or dispatch. No additional helper-local compilation context identified.

### 7. `policy_int_match_v1`
[0052:275](../../../crates/revaer-data/migrations/0052_indexer_search_result_ingest_proc.sql#L275); generated final-observed schema:16785.
Same branch structure as UUID, with `value_int` in EXISTS and integer equality; all function parameters end `_input`, with no competing table column or local.
H covers NULL/eq/empty-set/unsupported; P's populated/nonmatch/NULL-set/operator cases and `policy_helpers_sql` bind integer answers; N includes the helper in `populated-all-fields`. NULL equality is preserved, not coerced to FALSE. No extra helper-local context.

### 8. `policy_release_group_match_v1`
[0052:307](../../../crates/revaer-data/migrations/0052_indexer_search_result_ingest_proc.sql#L307); generated final-observed schema:16827.
Non-NULL token invokes text matching; TRUE short-circuits before the signal query. Absent token or non-TRUE match reaches EXISTS over `canonical_torrent_signal`, invoking text matching on `value_text` for qualifying rows.
`canonical_torrent_id`, `signal_key`, `value_text` are column-only names; every parameter ends `_input`. Neither nested call is dynamic; no local shares a signal column name.
H covers token success/failure/absence with an empty signal relation. P covers `release-token-match`, `release-token-false-persisted-true`, `release-token-and-persisted-false` and NULL operands; N covers token and persisted-signal regex errors plus successful fallback.
Token TRUE legitimately skips EXISTS; no qualifying rows skips its result-producing nested match. SQL may choose predicate evaluation order, so static eligibility does not claim an exact nested-call count. Both syntactic nested-call contexts already have witnesses.

### 9. `log_source_metadata_conflict_v1`
[0052:347](../../../crates/revaer-data/migrations/0052_indexer_search_result_ingest_proc.sql#L347); generated final-observed schema:9727.
COALESCE NULL text, independently truncate each value above 256, choose supplied time/`now()`, then three unconditional INSERTs: conflict, audit, health event. No authored nested helper, dynamic SQL or exception handler.
`existing_value`, `incoming_value`, `conflict_id` also name destination columns, but target lists are column positions and VALUES expressions have no source-row namespace. RETURNING names `source_metadata_conflict_id`, not local `conflict_id`; INTO is a variable target. No ambiguous expression was found.
[`compilation_cases` / `compilation_logger_outcome?` / `compilation_settings?`](../../../scripts/database_rebaseline/ingestion_compilation.rb) bind `cold-logger-setting` and `logger-first-setting`: direct NULL/clock-default, 257/258 truncation/fixed-clock, then nested cached reuse on a persisted source.
[`NativeFkExpectations`](../../../scripts/database_rebaseline/native_fk_expectations.rb) and [`NativeFkPhases#validate!`](../../../scripts/database_rebaseline/native_fk_phases.rb) bind the logger's five FK entries per invocation and exact operation adjacency/settings. This does not turn all caller conflict sites or failed FK inputs into newly witnessed paths.

### 10. `policy_action_to_decision_type`
[0093:4](../../../crates/revaer-data/migrations/0093_policy_action_decision_type_cast.sql#L4); generated final-observed schema:1056.
SQL IMMUTABLE CASE, no FROM, no PL/pgSQL compilation: four actions map directly; require/prefer map to flag; NULL falls through NULL. No variable/column conflict or nested authored call.
H names all six enum arms; P checks decision outcomes and assignment-cast inventory. N plus `NativePolicyCastScope` retain separately dispatched versus inlined cast evidence, rather than requiring a nonexistent reference callback.
Regex failure before decision insertion legitimately omits that ingestion cast; helper-first cast controls remain separate. No additional helper-local context identified.

## Bounded conclusion

No concrete uncovered helper-local conditional query, dynamic SQL path, nested authored callsite or variable/column ambiguity was identified in these ten current bodies. Unwitnessed permutations of scalar operands do not introduce another SQL binding context; this is not exhaustive input testing or universal native-provider certification.
No helper changes or additional matrix are proposed. The existing `ingestion complete conditional D3 proof` hard-fail stays; this note does not discharge parent-owned caller/trigger closure or authorize acceptance, cutover, merge or release.
Validation: read-only source comparisons and witness-method mapping only; no live tests, matrix replay or gates in this delegated audit. No production, dependency, observability or approval changes; rollback is removal of this support note.
Stale-policy check: read `AGENTS.md`, `.github/instructions/revaer-data.instructions.md`, ADR 569 D3 and the closure map. The map's execution-order text still asks to complete integrations its earlier rows report qualified; no policy or evidence-status edits were made in this one-file scope.
