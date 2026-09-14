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
- The last aggregate has 8,039 passing checks and a failing D3 completeness
  check. That count is not a completion percentage. Compact source-matching
  reports are prior execution evidence, not a new run or native trace.
- The bounded reconciliation records are retained as `items-1-4-df24ea2d.json`
  and `items-5-9-df24ea2d.json` in the checkpoint evidence archive. Raw traces
  were not re-reviewed or rerun to produce this map.

## Eleven-family reconciliation

| Audit family | Existing witnesses | Concrete remaining work |
| --- | --- | --- |
| 1. Mutating helper first | Compilation/logger fixtures use persisted source IDs, separate commits, NULL/length/clock controls. Metadata and GUID suites cover mutating-helper rollback/reuse. Producer provenance is repaired and live-qualified below. | Map logger-first compilation with the identity witnesses in row 5; add combinations only for an uncovered callsite/context. |
| 2. Populated policy helpers/cast | Policy matrix covers populated matches, actions, severities and preference branches. Twenty scoped native policy contexts are independently qualified. | Adopt the qualified native observation method in the canonical proof with exact source/context binding; archived qualification alone is not that adoption. |
| 3. Require rules/precedence | Require families, four scopes, rule order, trust boundaries, drop decisions, page omission and score/tag bounds are covered by the policy matrix. | No additional behavioral witness identified by this reconciliation. Retain its existing checks; native closure still follows row 2. |
| 4. In-call settings | Plain/observed compilation, settings-error and cancellation paths retain caller roles, backend identity and rollback-surviving setting observations. | Canonically bind the scoped native witnesses and preserve their exact success/error/rollback intervals. Three write observers are not every helper context. |
| 5. Discriminating identities | Wrapper identity and hash-fill cases cover creation, reuse, GUID promotion, explicit hash precedence and competing sources. Six empty/space-only input variants now have live new/reuse/promotion evidence. | Extend same-backend post-commit identity/reuse/fill evidence with exact D4 outcomes. Bind existing-wrapper reports before treating aggregate names as source-qualified evidence. |
| 6. Validation/rollback | 108 validation shapes in cold/helper-first modes cover request/instance states, title/identity, arrays, typed channels, invalid values and unchanged write images. | No additional behavioral witness identified. Reuse this matrix rather than commissioning another validation audit. |
| 7. Attributes/signals/native answers | Typed attributes, trust buckets/transitions, repeated signals, accented-title and malformed-magnet controls exist. The exact whitespace-only derivation now returns NULL before and after commit in both variants. Frozen suffix and NULL-distinct signal defects remain visible. | Observe K1's two actual local rank/bucket branches; equal final confidence does not establish that observation. |
| 8. Warm observation/scoring/wrapper | Separate source/score/title fixtures, seeder thresholds, GUID promotion and full-state wrapper oracles exist. Fresh dropped-result actions preserve an unrelated visible result without new page/best-context records. Six GUID-less/promoted equal/older/newer cases now pass cold/helper-first and post-commit qualification with exact source-versus-observation metadata. | Add discriminating post-commit score/title/wrapper outcomes. Fresh dropped-result cases do not prove every existing-source transition. |
| 9. Size/paging | Domain/cutoff controls, 1/2/3/26 sample cases, page boundaries and committed sampling are implemented. Twenty sampling variants now include 26 commits on one tested backend, independently checking newest retention and a late-arriving older sample. | Extend post-commit page-boundary/reuse; separately connected fixture calls are not a warm tested backend. |
| 10. Reachability | Controlled GUID interleavings and K2/K3 races execute the reachable cases. [R1/R2 support](569-ingestion-reachability.md) explains own-row score and signal-uniqueness exclusions. Prevent-merge cases retain actual unique-key errors. | Keep source dispositions distinct from live observations and incorporate their exact statement constraints into final closure. K1 still needs the observation in row 7. Do not invent successful split or signal-upsert paths. |
| 11. Catalog/native dependencies | Recursive catalogs pin defaults, constraints, indexes, read inputs, casts, dictionaries and native identities. Scoped native captures cover policy, logger, FK, wrapper and cancellation contexts. | Canonically integrate qualified observation/validation code, map remaining reachable callsites and skip conditions, and reject unexpected dispatch/definition drift. Installed callbacks and row-change eligibility are not execution traces. |

## Execution order

1. Completed: compilation report provenance and its dependency consumer passed
   4,836 targeted live checks. The later normalization/drop batch passed 1,509
   checks, including source-bound compilation/consumer checks again. Both
   disposable containers and volumes were removed; neither run claims D3.
2. Next: fill the remaining post-commit hash-fill/scoring/paging holes in rows 5, 8 and 9
   using the existing fixtures and complete-state oracles. Keep frozen D4
   failures distinct from independently asserted final success. Blank explicit
   hashes, whitespace derivation, fresh dropped-wrapper cases, temporal source
   freshness and committed 26th-sample retention are now qualified; do not
   repeat those investigations without relevant source drift. The two newer
   live drivers pass 245 and 261 checks including their prerequisites, not
   independent additive totals or complete D3 acceptance.
3. Complete K1 and canonical native observation adoption from rows 2, 4 and 11.
   Reuse the already qualified methods; rerun captures only where required by
   changed target definitions, compilation contexts or exact provenance.
4. Run the complete current-source D3 proof. Remove no completeness guard until
   every approved obligation has adequate evidence and all comparisons pass.

This closes only a database prerequisite. The operator association/root workflow,
single-init activation, UI scheduling, failure/recovery evidence, both Linux
package architectures, positive published Sonar coverage and GitHub review/merge
requirements remain in the [completion ledger](../564-media-completion-ledger.md).
Passing D3 alone cannot establish a usable service or authorize a merge.
