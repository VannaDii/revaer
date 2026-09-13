# ADR 569: R1/R2 ingestion reachability support

Scope: closure-map R1/R2 only; base `fa721b339b7ddf7508df9474032b80401eb491b9`.
Local source/constraint reasoning, not live validation, native traces, approval or D3 closure.
No migration, runtime, sentinel, gate, policy or dependency changes; parent owns live qualification.

## Citation key and frozen authority

All migration citations below are under `crates/revaer-data/migrations/`:
- S = `0052_indexer_search_result_ingest_proc.sql`; C = `0022_indexer_canonicalization.sql`.
- T = `0012_indexer_core.sql`; I = `0014_indexer_instances.sql`.
- Q = `0023_indexer_search_requests.sql`; B = `0024_indexer_scoring.sql`.
- N = `0066_indexer_proc_fixes.sql`; V = `0092_search_result_ingest_variable_conflict.sql`.
`config/database-rebaseline.env:2-7` pins 167 migrations, the corpus/candidate/final digests and 1,624 statements.
The frozen installed title helper is N:3-45, replacing S:20-62; its NULL guard is N:23-24.
V:2-28 sets ingestion `use_column`; this matters for S:1914-1921's selected maximum.
These dispositions assume the pinned schema, enabled constraints, unmodified frozen triggers and successful preceding statements.
Concurrent counterexamples use READ COMMITTED and an explicitly identified direct writer, not an invented runtime API.
No caller privilege, isolation or compiler-setting change is proposed; stronger-isolation retries are not these cases.

## R1 dispositions

| Site | Disposition and proof |
| --- | --- |
| S:40-41 (now N:23-24) | NULL title return is unreachable **through ingestion**, including concurrency: S:677-684 rejects NULL/blank, S:771-783 passes a non-NULL local string. Another transaction cannot replace that local. Direct `normalize_title_v1(NULL)` remains a valid helper case. |
| S:80-81 | Blank-magnet return is unreachable **through ingestion**: S:689 maps blank to NULL; S:731-734 passes that local to derivation, which returns before normalization at S:148-149. Direct `normalize_magnet_uri_v1(' ')` reaches it. |
| S:153-154 | Normalized-URI NULL return is unreachable **through ingestion**: a surviving non-NULL URI is nonblank; every such normalizer return is non-NULL (S:84-122). Direct `derive_magnet_hash_v1(NULL,NULL,' ')` reaches it. |
| S:655-663 | **Reachable, even serially.** I:114 is nullable and is an enum column, not an FK to `trust_tier`; I:100-132 has no such FK. NULL key skips the lookup with initial rank 0 (S:471); a non-NULL key without a T row takes S:660-661. T:79-88 makes a present rank non-NULL, not the row mandatory. Next case K1. |
| S:1480 false | Unreachable: Q:93-94 requires a snapshot. S:598-608 copies it with the request ID and rejects missing requests. A later concurrent update/delete cannot null the copied variable. |
| S:1924-1925 | Unreachable after successful S:1883-1911: the inserted/updated current score satisfies S:1917-1919 and has non-NULL source ID (B:46-48). The transaction sees its own row; its write lock prevents a concurrent delete/update, including a cascading delete, from removing it before this lookup. |
| S:1927-1928 true | Unreachable: the selected maximum includes that same locked current score, hence `best_context_score >= score_total_context`. The latter cannot exceed the maximum by 2. Other writers can alter other rows, not this witness; both scores are non-NULL (B:49-50). |
| S:2758 false | `COUNT(*)` cannot be NULL, but **zero is concurrently reachable**. S:2740 can skip a duplicate without retaining a tuple write lock; pruning at S:2742-2750 is not a lock on its retained rows. A writer can delete those rows before S:2752-2756. Even a fresh insert can be pruned if older than 25 retained samples. Next case K3. |

The direct helper calls above return NULL but do not establish in-ingestion helper dispatch.
Existing NULL-title validation (`scripts/database_rebaseline/ingestion_validation.rb:68-74`) instead proves rejection.
Successful own-row INSERT/UPDATE is a concurrency witness for the score rows, not for every sample outcome.

## R2 durable attributes

C:265-304 requires a non-NULL `(source_id, attr_key)` unique pair, exactly one value and its key-specific channel.
The following eleven UPDATE arms are **serially excluded, concurrently reachable with a direct writer (K2)**:

| Attribute | Guard / conflict UPDATE in S |
| --- | --- |
| tracker_name | 2182-2196 |
| tracker_category | 2209-2223 |
| tracker_subcategory | 2236-2250 |
| size_bytes_reported | 2263-2277 |
| files_count | 2281-2295 |
| imdb_id | 2299-2313 |
| tmdb_id | 2326-2340 |
| tvdb_id | 2353-2367 |
| season | 2380-2394 |
| episode | 2398-2412 |
| year | 2416-2430 |

S:2116-2180 reads all eleven values before any insert. A visible existing row cannot supply NULL under C:289-304.
For same-source ingestion peers, S:1171-1205 writes the source before those reads and serializes their refreshes.
A newly inserted source (S:965-1004) is not available to another committed-FK child writer before commit.
This does **not** exclude all writers: on an existing identity with no hash/GUID fill, S:1171-1205 updates only
last-seen fields and `updated_at`, not keys (C:173-177,194-196,220-222). Its NO KEY UPDATE lock is compatible
with the KEY SHARE needed by a direct child insert's FK (C:267-269). Missing-value reads have no gap lock.
Under READ COMMITTED, a pending child insert is invisible to those reads but can become the upsert conflict.
Each resulting UPDATE keeps the writer's non-NULL value via COALESCE; its EXCLUDED fallback stays excluded.
Stable values/IDs alone therefore cannot prove UPDATE execution; K2 needs the parent's actual UPDATE observation.

## R2 signals

All six UPDATE arms are **constraint-unreachable, serially and concurrently**:
release_group S:2435-2453; language S:2456-2474; subtitles S:2477-2495;
year S:2498-2516; season S:2519-2537; episode S:2540-2558.
C:352-368 has nullable channels without defaults, exactly one populated channel and ordinary NULL-distinct
UNIQUE `(canonical_torrent_id, signal_key, value_text, value_int)`. Each insert omits the other channel.
Thus every legal candidate has a NULL in this arbiter, so even an identical concurrent row cannot conflict.
Primary-key collisions are not this conflict target. No constraint/index change is a valid coverage fixture.

## Minimal next cases for the parent (not run here)

K1: Reuse the smallest valid non-ID ingestion fixture with `language_primary='en'`, a fresh backend per frozen first call.
Case A sets the instance trust key NULL. Case B retains key `public` but removes that trust-tier row in the
disposable fixture with no `search_profile_trust_tier` references (`0016_search_profiles_torznab.sql:36-47`).
No constraint needs disabling. Both must reach bucket 0 and language confidence 0.5 (S:665-675,2456-2474).
Observe the skip versus S:660-661 fallback separately. A concurrent deletion committed before S:656 is also valid.

K2: Seed an existing canonical/source with matching supplied hashes/GUID, no durable attrs and valid request.
Writer B begins and inserts all eleven valid typed attrs, leaving them uncommitted; choose its values distinct
from A's incoming values (valid IMDb/TMDB/TVDB values). Start wrapper A with all eleven attrs; its non-key
refresh passes B's FK lock, reads all eleven as absent, then waits on B at the tracker_name conflict.
After observing that owned wait, commit B. Assert eleven UPDATE executions, retained B values/IDs and no
differing-value logger writes. Run a non-ID eight-attribute control separately: frozen all-ID A later fails
D5 `42P10` at S:2561 onward and rolls back its work; approved final succeeds. Retain rollback-surviving traces.

K3: Seed an existing hashed canonical/source plus 26 samples, distinct observed times, all size 1024.
Writer B begins and row-locks all 26 samples FOR UPDATE without changing them. A ingests the newest exact
sample tuple (explicit observed time, positive allowed size, no ID attrs): S:2740 skips that duplicate.
A's pruning DELETE targets the oldest sample and waits on B. After observing this wait, B deletes all 26
and commits. A skips the now-deleted victim, counts zero, and omits the rollup write. Assert zero samples,
unchanged prior rollup and actual S:2758 false observation. Do not substitute deletion before the duplicate check.

## Evidence, risk and retained limits

Existing claims remain qualified: `scripts/database_rebaseline/ingestion_metadata.rb:44-50` explicitly lacks
concurrency; `scripts/database_rebaseline/ingestion_attributes.rb:129-132` disclaims signal UPDATE/full D3.
Existing `source_frozen` defects remain visible, including literal-backslash suffix recognition and duplicate
signals without confidence increase. D4 `42P07` and D5 `42P10` remain distinct approved final
differences, never frozen successes (`scripts/database_rebaseline/ingestion_approved_deltas.rb:6-15`).
Added only existing-pattern schema/statement-order assertions to `scripts/tests/database-ingestion-metadata-test.rb`.
Focused checks on the base plus only the two additions named here, Ruby 4.0.6, all exit 0:
- `just --command ruby --disable-gems scripts/tests/database-ingestion-metadata-test.rb`: 10,614 assertions; 10,104 semantic mutations rejected.
- `just --command ruby --disable-gems scripts/tests/database-ingestion-attributes-test.rb`: 206 assertions.
- `just db-rebaseline-freeze`: exact pinned 167-migration corpus passed; `git diff --check`: passed.
These are local unit/source checks, not database executions. No live service, native tracing, full CI or UI run.
Reviewed root `AGENTS.md`, database scoped rules and relevant devops proof rules; no policy correction made.
Observability: support note and local assertion output only. Risk: source reasoning is not scheduler evidence;
rollback is removal of these test/note additions. No new dependencies or architectural decision.
Residual: parent must execute K1-K3 and integrate native/cold/warm evidence; no full D3 closure is claimed.
