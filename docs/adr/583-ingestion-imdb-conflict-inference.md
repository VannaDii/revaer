# Ingestion IMDb conflict inference

- Status: Proposed
- Date: 2026-09-10
- Operator approval: Pending; D5 is not covered by conditional D3 or held D4
- Context:
  - ADR 582 reproduces SQLSTATE `42P10` for real IMDb ingestion on a fresh
    backend against both the frozen reference and the final candidate. The
    tested call rolls back rather than committing the external identifier.
  - `canonical_external_id_text_uq` is a unique index over
    `(canonical_torrent_id, id_type, id_value_text)` with predicate
    `id_value_text IS NOT NULL`. The IMDb conflict target omits that predicate.
- Decision:
  - Proposed D5: in the final candidate's `public.search_result_ingest_v1`
    IMDb insert only, add `WHERE id_value_text IS NOT NULL` immediately after
    `ON CONFLICT (canonical_torrent_id, id_type, id_value_text)` and before
    `DO UPDATE SET`. Preserve the existing index, values and update assignments.
  - Authorize this one additional routine-definition and behavioral parity
    delta only after explicit operator approval. Keep all 167 frozen migration
    files unchanged. No implementation is made by this proposal.
- Consequences:
  - The final variant should use the existing partial unique index instead of
    raising `42P10`; the untouched reference must retain the counterexample.
    This expected difference cannot be hidden under D3's general parity check.
  - D4 remains separately held. Two integer-ID ingestion targets and two
    canonical-merge targets have analogous source structure but are outside
    D5 and were not independently reproduced by ADR 582. This proposal does
    not certify all external-ID paths or the complete D3 contract.
- Follow-up:
  - Obtain the exact D5 decision, then extend the final-delta guard, independent
    expected outcomes and immutable final digest together. Do not activate init
    or retire migrations until the complete accepted cutover proof passes.

## Task Record

### Later Isolated Evidence

The operator's 2026-09-11 revised goal explicitly permits unpublished
nonproduction decision experiments. The resulting
[consolidated package](588-first-release-decision-package.md#database-reproduced-defect-family)
reproduces the IMDb failure and the analogous TMDB/TVDB ingestion failures on
both untouched reference and current final SQL. Its temporary candidate fixes
all three with matching partial-index predicates. That package proposes an
explicit scope extension for review; this original IMDb-only proposal is not
silently broadened or approved. Canonical-merge changes remain outside it.
The experiment is not complete D3 proof or production implementation.

- Motivation:
  - Present the newly observed production-path defect before changing the
    approved finalization boundary.
- Design notes:
  - Prefer matching the existing partial index over changing its uniqueness,
    null semantics, schema or privileges. Rebuilding the index, ignoring the
    SQLSTATE, excluding IMDb, or changing the reference are not authorized.
- Test coverage summary:
  - Existing evidence: successful real fixture ingestion followed by a fresh
    IMDb call, complete role/settings/result/table evidence and shared `42P10`.
  - Required after approval: first insert, repeated value upsert, normalized
    identifier case, distinct identifiers, trust-rank and last-seen updates,
    invalid/null input, rollback and constrained runtime permissions. Retain
    the reference failure and verify exact expected final writes independently.
    Run the complete final proof, `just ci`, `just ui-e2e` and applicable remote
    checks. No proposed-fix success is claimed.
- Observability updates:
  - Preserve raw diagnostics and all before/after images; label only the exact
    approved difference in the proof report. No runtime suppression is proposed.
- Status-doc validation:
  - D3 remains incomplete and the final init inert. This proposal adds no
    accepted decision and does not remove any included first-release feature.
- Risk & rollback plan:
  - Overbroad conflict changes could alter canonical identity. Restrict the
    accepted delta to one known insert and enforce exact surrounding SQL. If
    validation fails, do not publish or activate the candidate; retain the
    current digest and evidence. Resolve analogous paths separately with proof.
- Dependency rationale:
  - No new dependency, index or privilege is proposed.
- Stale-policy check:
  - Reviewed root AGENTS, scoped data/devops instructions, ADRs 569/579/582 and
    the frozen index/routine definitions. No criteria or architecture approval
    is inferred. Existing records keep their exact acceptance boundaries.
