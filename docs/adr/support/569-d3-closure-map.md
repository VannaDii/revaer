# D3 closure map

Status: In progress. This reconciles the eleven families in the retained
2026-09-11 closure audit against checkpoint `df24ea2d` and the follow-up changes
recorded in [ADR 569](../569-init-privilege-and-timeout-resolution.md).
It is an execution map, not a new architectural decision or D3 acceptance.

## Authority and evidence

- [ADR 569 D3](../569-init-privilege-and-timeout-resolution.md#d3-function-local-variable-resolution)
  requires independent cold/warm ingestion, reachable helper/trigger, result,
  mutation, error and setting-scope evidence under the pinned PostgreSQL roles.
- ADR 588's exact D4/D5 differences remain separate accepted differences. A
  frozen failure is never relabeled as a successful final/reference comparison.
- The original audit proposes witnesses. It does not independently authorize
  additional behavior, change acceptance criteria or demand every Cartesian
  combination of unrelated inputs. A missing combination needs a new test when
  it exercises an otherwise unproven branch, callsite or compilation context.
- The latest 2026-09-14 full-method replay has 9,851 passing checks and a failing D3 completeness
  check. That count is not a completion percentage. Compact source-matching
  reports are prior execution evidence, not a new run or native trace. The
  settings and warm-helper groups also pass within this complete-method replay;
  the final proof itself remains `completed=false` and `passed=false`.
- The bounded reconciliation records are retained as `items-1-4-df24ea2d.json`
  and `items-5-9-df24ea2d.json` in the checkpoint evidence archive. Raw traces
  were not re-reviewed or rerun to produce this map.

## Eleven-family reconciliation

| Audit family | Existing witnesses | Concrete remaining work |
| --- | --- | --- |
| 1. Mutating helper first | Compilation/logger fixtures use persisted source IDs, separate commits, NULL/length/clock controls. Metadata and GUID suites cover mutating-helper rollback/reuse. Producer provenance is repaired and live-qualified below. | Source mapping is recorded in [Rows 1 and 5 source bindings](#rows-1-and-5-source-bindings); retain its distinct compilation contexts and live-evidence limits. Add combinations only for an uncovered callsite/context. |
| 2. Populated policy helpers/cast | Policy matrix covers populated matches, actions, severities and preference branches. [Canonical native policy integration](../569-init-privilege-and-timeout-resolution.md#canonical-native-policy-integration-2026-09-14-in-progress) now passes all twenty source-bound plain/observed contexts and 304 checks in the full proof, with unchanged source/target identities and successful cleanup. | Retain the canonical observer and independent helper/regex/inlined-cast validators. Do not repeat this matrix without relevant source/context drift; it does not close other native callsites or complete D3. |
| 3. Require rules/precedence | Require families, four scopes, rule order, trust boundaries, drop decisions, page omission and score/tag bounds are covered by the policy matrix. | No additional behavioral witness identified by this reconciliation. Retain its existing checks; native closure still follows row 2. |
| 4. In-call settings | Plain/observed compilation and settings-error paths retain caller roles, backend identity and rollback-surviving setting observations. Four cancellation pairs pass in the full-method replay. [Native settings integration](../569-init-privilege-and-timeout-resolution.md#native-settings-integration-2026-09-14-in-progress) additionally passes twelve cold/helper-first variant pairs and 45 producer checks with exact success/error/rollback intervals, source/target identities and cleanup. | The twelve settings and four warm-helper pairs now pass in the complete-method replay. Finish the bounded remaining callsite/skip bindings. These bounded observations do not establish every callsite or FK count. |
| 5. Discriminating identities | Wrapper identity and hash-fill cases cover creation, reuse, GUID promotion, explicit hash precedence and competing sources. Six empty/space-only input variants have live new/reuse/promotion evidence. The 24 committed hash-fill variants now qualify warm/fill/reuse on one backend, preserving conflicts, complete state and five sequence values, including frozen failed-call allocations. | Existing-wrapper report bindings and logger-first distinctions are mapped in [Rows 1 and 5 source bindings](#rows-1-and-5-source-bindings), not newly live-qualified. Do not repeat the qualified committed hash-fill matrix without relevant source drift. |
| 6. Validation/rollback | 108 validation shapes in cold/helper-first modes cover request/instance states, title/identity, arrays, typed channels, invalid values and unchanged write images. | No additional behavioral witness identified. Reuse this matrix rather than commissioning another validation audit. |
| 7. Attributes/signals/native answers | Typed attributes, trust buckets/transitions, repeated signals, accented-title and malformed-magnet controls exist. The exact whitespace-only derivation returns NULL before/after commit in both variants. [Canonical K1 integration](../569-init-privilege-and-timeout-resolution.md#canonical-native-k1-integration-2026-09-14-in-progress) now passes eight paired native contexts, 24 calls and 384 events in the full proof, with separate rank-40 calibration, exact source/target identity and cleanup. Frozen suffix and NULL-distinct signal defects remain visible. | Retain canonical K1 registration and its independent validator. Do not repeat this matrix without relevant source/context drift; it does not close other native callsites or complete D3. |
| 8. Warm observation/scoring/wrapper | Separate source/score/title fixtures, GUID promotion, dropped results and temporal source freshness are qualified. Eight committed scoring variants now preserve distinct stored scores, the 99/100-seeder boundary, ranked title refresh, wrapper-selected identity and nine sequence values across four tested commits. | Retain current-source qualification and map these witnesses to the remaining native/callsite obligations. Fresh dropped-result cases do not independently prove every existing-source transition. |
| 9. Size/paging | Domain/cutoff controls, 1/2/3/26 sample cases, page boundaries and committed sampling are implemented. Twenty sampling variants include 26 commits on one tested backend. Eight committed paging variants now qualify the tenth/eleventh-item boundary and new/original-item reuse with all 18 sequence values and exact frozen D4 outcomes. | No further serial page-boundary witness identified here. Retain current-source qualification; these cases do not establish concurrent paging or native dispatch. |
| 10. Reachability | Controlled GUID interleavings and K2/K3 races execute the reachable cases. [R1/R2 support](569-ingestion-reachability.md) explains own-row score and signal-uniqueness exclusions. Prevent-merge cases retain actual unique-key errors; K1 readback is canonically integrated in row 7. | Keep source dispositions distinct from live observations and incorporate their exact statement constraints into final closure. Do not invent successful split or signal-upsert paths. |
| 11. Catalog/native dependencies | Recursive catalogs pin defaults, constraints, indexes, read inputs, casts, dictionaries and native identities. K1, policy and [eight FK contexts](../569-init-privilege-and-timeout-resolution.md#canonical-native-fk-integration-2026-09-14-in-progress) pass in the canonical proof. FK counts, ownership and settings are bound to independent source predictions. Four cancellation pairs pass in the full-method replay; twelve settings and four warm-helper pairs pass in the current full-method replay. All retain separate cleanup evidence. | Finish the bounded remaining callsite/skip dispositions and reject unexpected dispatch/definition drift. Installed callbacks and row-change eligibility are not execution traces. Do not repeat integrated matrices without relevant source/context drift. |

## Rows 1 and 5 source bindings

Source inspection at detached `19d59c7e` maps definitions, entrypoints and validator contracts only.
No retained report or live witness was requalified; D3 completeness and all holds remain unchanged.

- **Direct versus wrapper:** [`ingestion_call`](../../../scripts/database_rebaseline/ingestion_proof.rb) emits `public.search_result_ingest_v1`;
  [`correction_session`](../../../scripts/database_rebaseline/ingestion_corrections.rb) selects `public.search_result_ingest` only with `wrapper: true`.
  [`wrapper_cases`](../../../scripts/database_rebaseline/ingestion_wrapper.rb) therefore includes direct `stored-score-*` controls alongside actual wrapper cases; the report family name alone is not an entrypoint.
- **Logger compiled first:** [`compilation_cases` / `compilation_call`](../../../scripts/database_rebaseline/ingestion_compilation.rb) map `logger-first-setting` to two direct `log_source_metadata_conflict_v1` mutations on a persisted fixture source, then direct v1 ingestion with `b*40`.
  NULL values/clock and 257/258-character values/fixed time exercise separate committed calls on one fixture-separated backend, priming the logger under caller `error` before ingestion reuses it.
  `cold-logger-setting` instead first reaches the logger inside v1; the changed infohash and derived magnet hash produce two nested logger calls.
  `compilation_verify!`, `compilation_logger_outcome?` and `compilation_settings?` bind state, payloads, roles/clocks and ordered write observations: frozen ingestion has ambient `use_column`, final `error`.
  Ambient in-call settings are not the cached helper's compilation mode. This committed logger-first case is not mutating-helper rollback/reuse or a wrapper identity case.
- **Identity and fill:** [`wrapper_identity_cases`](../../../scripts/database_rebaseline/ingestion_identity.rb) registers `identity-<spec>-new`, `-reuse`, `-promote-guid`, including v2 precedence, explicit magnet, blank inputs and title/size fallback, all through the wrapper.
  `wrapper_outcome?` delegates to `wrapper_identity?`, `wrapper_identity_preserved?` and `wrapper_identity_prior_selection?` for selected identities, hashes, relationships and untouched decoys.
  [`wrapper_hash_fill_cases` / `hash_fill_verify!`](../../../scripts/database_rebaseline/ingestion_hash_fill.rb) bind `fill-{v1,v2,magnet}-{competing,uncontested}` to independently specified fixture/read/write states, conflicts and returned identities.
  `wrapper_isolated` runs each in `cold`, `helpers-first` (helper prelude), and `warm-rollback` (real first call rolled back, same-backend retry committed); `wrapper_verify!` checks backend separation, continuity, roles/settings and D4 lifetime.
  These modes do not mean logger-first mutation, nor successful frozen post-commit reuse; row 5's committed hash-fill witnesses remain separate.
- **Exact report binding:** producers retain `ingestion-compilation/run-*/report.json` and `ingestion-wrapper/run-*/report.json` plus per-case JSON.
  [`dependency_observation_cases`](../../../scripts/database_rebaseline/ingestion_dependencies.rb) enumerates `<compilation-case>-<variant>-observed` and every `<wrapper-case>-<mode>-<variant>`: declared compilation call counts, one cold/helper-first frame, two warm-rollback frames.
  `dependency_read_observation` requires current-process validated byte hashes; `dependency_observations` also checks successful producer checks, candidate/final hashes, PostgreSQL image and `compilation_source_hashes` / `wrapper_source_hashes`.
  `dependency_observation_frames` rejects missing/duplicate frames, failed calls and stale roles. `dependency_observations` returns FK-input classifications with `callback_entry_trace: false`; compilation setting events are not native entries.
- **Definition/native boundary:** [`NativeFkExpectations`](../../../scripts/database_rebaseline/native_fk_expectations.rb) binds exact source files and the signatures/body hashes/settings of logger, v1 and wrapper; declared roots are separately checked by `dependency_validate_declared_roots!`.
  [`NativeFkPhases#validate!`](../../../scripts/database_rebaseline/native_fk_phases.rb) binds the two compilation scenarios to operation roots, logger adjacency, settings and independent callback multisets (cold 19; logger-first 5/5/19), not counts inferred from wrapper table changes.
  The expectation set separately names `existing-v2-hash-conflict-warm-rollback`; that one native scenario must not be generalized to all identity/fill cases.

No missing producer/report-consumer binding for these named identity/fill cases was substantiated by this scoped trace.
Remaining live/native qualification must bind the exact applicable reports and contexts; this mapping neither adds a matrix nor substitutes for warm-observer qualification or the complete proof.

The subsequent [warm-helper integration](../569-init-privilege-and-timeout-resolution.md#native-warm-helper-integration-2026-09-14-in-progress)
qualifies the remaining magnet-URI and title-size cold/committed-warm observations
in four reference/final plain/observed pairs (73 producer checks). It preserves
complete application comparisons, exact identity answers, read inputs, D4
diagnostics, current-source/target identity and independent cleanup. The complete
method replay also qualifies these four pairs and all twelve settings pairs.
This closes their integration qualification in rows 4 and 11, not unmapped
callsites/skip conditions or D3 completeness. No missing source-to-report binding was
identified for rows 1 and 5 by the bounded mapping above.

## Execution order

1. Completed: compilation report provenance and its dependency consumer passed
   4,836 targeted live checks. The later normalization/drop batch passed 1,509
   checks, including source-bound compilation/consumer checks again. Both
   disposable containers and volumes were removed; neither run claims D3.
2. Completed: the identified post-commit hash-fill, scoring/title and paging
   witnesses use existing fixtures and complete-state oracles. Keep frozen D4
   failures distinct from independently asserted final success. Blank explicit
   hashes, whitespace derivation, fresh dropped-wrapper cases, temporal source
   freshness and committed 26th-sample retention are now qualified; do not
   repeat those investigations without relevant source drift. The two newer
   live drivers pass 245 and 261 checks including their prerequisites, not
   independent additive totals or complete D3 acceptance.
   Committed hash-fill and paging now pass 24 and eight variants, respectively,
   with 249 and 233 driver checks including the shared prerequisites. Their
   [integration record](../569-init-privilege-and-timeout-resolution.md#committed-hash-fill-and-paging-2026-09-14-in-progress)
   retains the initial failed hash-counter expectation and successful cleanup.
   Eight committed scoring/title variants also pass, with 233 driver checks
   including shared prerequisites and 1,032 focused unit assertions.
3. Complete canonical native observation adoption from rows 2, 4, 7 and 11.
   K1's two local trust-rank branches now have qualified native readback,
   including all three actual calls and the separate nonzero calibration.
   Reuse the already qualified methods; rerun captures only where required by
   changed target definitions, compilation contexts or exact provenance.
   [Shared process/tool preparation](../569-init-privilege-and-timeout-resolution.md#shared-native-observer-tooling-2026-09-14-in-progress)
   now has registered policy tests and verified cached/fresh-build inputs.
   Native sessions and K1 readback are now integrated into `FinalProof`, with
   current-source success in the complete run and separate cleanup evidence.
   The qualified policy observer is also canonically integrated: all twenty
   paired contexts pass in the full run with independent source binding and
   cleanup. The four FK scenarios now also pass all eight paired contexts and
   115 producer checks in the complete run, with exact source predictions and
   separate cleanup. Cancellation now uses the canonical controller in four
   plain/observed pairs in the full-method replay, with exact setting,
   rollback/recovery, independent clocks and successful cleanup. Complete the
   settings and warm-helper integrations in the complete proof, and bind the
   remaining callsites/skip conditions; the completed native groups do not
   discharge those obligations. The initial full-run server disconnect
   and successful serial replay remain separately recorded, not conflated.
4. Bind the remaining callsite/skip dispositions to the current source and the
   9,851 passing checks. The full-method replay has executed and rejected only
   its completeness guard; that is not a pass. Remove no guard until every
   approved obligation has adequate evidence and all comparisons pass, then
   rerun the complete proof. Add a witness only for a concrete uncovered path.

This closes only a database prerequisite. The operator association/root workflow,
single-init activation, UI scheduling, failure/recovery evidence, both Linux
package architectures, positive published Sonar coverage and GitHub review/merge
requirements remain in the [completion ledger](../564-media-completion-ledger.md).
Passing D3 alone cannot establish a usable service or authorize a merge.
