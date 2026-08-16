# Generation-fenced media job snapshot reader contract

- Status: Proposed
- Date: 2026-08-16
- Operator approval: Pending

## Problem

- Accepted ADR 446 selects a narrow claim followed by bounded snapshot readers,
  but it intentionally leaves the exact stored-procedure ABI, cardinalities,
  transaction boundary, errors, and aggregate identity undecided.
- Jobs already capture normalized configuration rows, but a worker can reach
  only partial claim fields and desired-stream rows. Live-profile reads or
  process constants would violate immutable retry behavior.
- A set of independently successful reads is not sufficient. The worker must
  prove that every required family belongs to one immutable job snapshot and
  that its attempt and claim generation are still current.
- Proposed ADRs 507-516 may add accepted metadata, chapter, stream, video,
  recovery, retention, lifecycle, audio, or discovery state later. Their
  pending recommendations are not part of an approved snapshot merely because
  this transport can carry versioned extensions.

## Options

1. **Expand the claim row.** Return every scalar and collection from the claim
   procedure. This reopens the option rejected by ADR 446 and makes claim
   evolution and bounded collection handling difficult.
2. **Use generation-fenced family readers in one coherent transaction.** Keep
   claim ownership narrow, load each typed family through stored procedures,
   and verify one persisted aggregate identity before compilation.
3. **Read each family independently without an aggregate identity.** This is
   simpler, but a missing family, duplicate row, stale generation, or partial
   snapshot can be mistaken for a valid empty policy.

## Recommendation

- Adopt option 2.
- Every reader accepts exactly `job_public_id_input UUID`,
  `attempt_number_input INT`, and `claim_generation_input BIGINT`. It first
  validates that the tuple identifies the current claimed attempt. Readers do
  not accept a profile id, live policy id, live target id, path, or caller-
  supplied row count.
- Use these versioned procedures for the base aggregate:
  - `media_job_policy_snapshot_header_get_v1`
  - `media_job_policy_snapshot_filter_get_v1`
  - `media_job_policy_snapshot_behavior_get_v1`
  - `media_job_policy_snapshot_root_list_v1`
  - `media_job_policy_snapshot_file_rule_list_v1`
  - `media_job_policy_snapshot_subtitle_rule_list_v1`
  - `media_job_policy_snapshot_retention_rule_list_v1`
  - `media_job_policy_snapshot_compatibility_target_list_v1`
  - `media_job_policy_snapshot_operation_cost_list_v1`
  - `media_job_policy_snapshot_classification_rule_list_v1`
  - `media_job_policy_snapshot_maintenance_window_list_v1`
  - `media_job_target_snapshot_header_get_v1`
  - `media_job_target_snapshot_stream_list_v1`
- The header returns one `snapshot_public_id`, `snapshot_contract_version`,
  profile configuration version, policy key and version, target key and
  version, capture timestamp, every expected family count, canonical aggregate
  byte count, and lowercase SHA-256 aggregate digest. It returns no mutable live
  profile state.
- Scalar readers return exactly one row. List readers return all rows, including
  disabled rows, in canonical order. The caller may not interpret a truncated
  page as a complete snapshot.

### Cardinality And Ordering

| Family | Required wire cardinality | Canonical order |
| --- | --- | --- |
| Header | exactly 1 | n/a |
| Filter | exactly 1 | n/a |
| Behavior | exactly 1 | n/a |
| Managed roots | exactly 5 after separate ADR 523 approval | root-kind ordinal |
| File rules | 0 through 128 | `sort_order`, normalized rule key |
| Subtitle discovery rules | 0 through 4; semantic zero-rule handling belongs to ADR 524 | precedence, normalized pattern |
| Retention rules | 0 through 128 | `sort_order`, normalized rule key |
| Compatibility targets | 0 through 32 | `sort_order`, target key, version |
| Operation costs | 0 through 13 on the wire; completeness belongs to ADR 518 | operation-kind ordinal |
| Classification rules | 0 through 128 | `sort_order`, normalized rule key |
| Maintenance windows | 0 through 32 | day, start, end, `sort_order` |
| Target header | exactly 1 | n/a |
| Target streams | 1 through 64 | `sort_order`, stream key |

- Bounds count enabled and disabled rows. Duplicate sort positions, duplicate
  normalized keys, a count different from the header, or a row beyond a bound
  invalidates the aggregate rather than being ignored.
- Proposed ADRs 507-511 and 515 may add normalized target child families only
  after separate approval. Each accepted family requires an explicit reader,
  count, bound, canonical family ordinal, and snapshot-contract version change.
  Proposed ADRs 512-514 and 516 describe operational state, not immutable job-
  policy rows, and do not enter this aggregate by implication.

### Isolation And Fence Handling

- Load the complete aggregate in one short `REPEATABLE READ, READ ONLY`
  transaction after claim. The Rust data adapter owns the transaction and calls
  only the procedures above; it does not issue inline row queries.
- Every reader repeats the current attempt-and-generation check inside that
  transaction. A mismatched tuple returns an error, not zero rows.
- The transaction commits before pure compilation. Immediately after
  compilation and before the first filesystem, native-process, lease, or audit
  side effect, the worker calls a generation-fenced claim assertion in a fresh
  transaction. All later worker writes remain independently fenced.
- A claim that changes while pure loading or compilation is in progress causes
  the fresh assertion to fail. The discarded in-memory value creates no durable
  evidence and starts no side effect.

### Aggregate Identity

- `snapshot_contract_version = 1` uses a domain-separated binary frame beginning
  with the ASCII bytes `revaer-media-job-snapshot`, a zero byte, and an unsigned
  32-bit version.
- Families follow their fixed ordinal. Each family contains an unsigned 32-bit
  row count; each row contains fixed field ordinals. Integers are fixed-width
  big-endian, booleans are one byte, nullable fields use a one-byte presence
  marker, and UTF-8 text uses an unsigned 32-bit byte length followed by bytes.
  No locale, database surrogate id, timestamp rendering, JSON, YAML, or map
  iteration order participates.
- The digest covers all semantic scalar and child values, their enabled states,
  stable public or normalized keys, profile/policy/target versions, and family
  counts. It excludes capture time, mutable claim state, database row ids,
  capability identity, source inspection, and host paths that ADR 523 treats as
  separate resolved-root evidence.
- Snapshot creation persists the framed byte count and SHA-256 digest in the
  same transaction as the child rows. The bounded Rust loader independently
  frames the returned rows and requires exact byte-count and digest equality.
  Existing `pgcrypto` and Rust SHA-256 support are sufficient; no dependency is
  proposed.

### Stable Errors

- The procedures and adapter expose these stable codes without embedding paths
  or row values:
  - `media_snapshot_claim_not_current`
  - `media_snapshot_header_missing`
  - `media_snapshot_scalar_missing`
  - `media_snapshot_family_count_mismatch`
  - `media_snapshot_family_bound_exceeded`
  - `media_snapshot_duplicate_identity`
  - `media_snapshot_contract_version_unsupported`
  - `media_snapshot_aggregate_size_mismatch`
  - `media_snapshot_aggregate_digest_mismatch`
  - `media_snapshot_claim_lost_after_compile`
- The structured error also carries one bounded family enum and expected and
  actual counts when applicable. It never falls back to a live profile,
  hard-coded default, empty collection, or latest target version.

## Consequences

- Worker planning receives one complete, reproducible aggregate and can prove
  omissions or stale ownership before side effects.
- Multiple bounded reads add database round trips, but they occur in one short
  transaction and preserve ADR 446's narrow claim contract.
- The database and Rust implementations must share canonical framing test
  vectors. A framing or semantic change requires a new contract version rather
  than reinterpretation.
- Cross-family semantic validation remains owned by the pure compiler proposed
  in ADR 518. This ADR validates transport completeness only.

## Implementation Boundary

- This proposal authorizes no implementation while its status is `Proposed`.
- Acceptance would authorize only the reader names and arguments, cardinality
  and ordering bounds, repeatable-read loading protocol, aggregate framing and
  digest, stable errors, and post-compile fence assertion described above.
- Acceptance alone would not authorize the exact five-root content in ADR 523,
  the policy semantics in ADR 518, capability binding in ADR 519, re-plan in ADR
  520, operator resources in ADR 521, or any recommendation in proposed ADRs
  507-516.
- Persistence changes, if later approved, belong in the pre-v1 `init.sql` under
  the separately approved transition contract. Runtime access remains stored-
  procedure-only and collaborators remain injected.
- No schema, SQL, Rust, API, workflow, generated contract, or runtime behavior
  may change until this ADR receives explicit decision-specific approval and all
  required cross-ADR contracts are accepted.

## Validation

- Proposal validation is documentation-only.
- After approval, add database and Rust known-answer vectors for empty, minimum,
  and maximum families; every scalar and nullable type; Unicode text; row-order
  changes; and one-bit value changes.
- Add concurrency tests for claim loss before the first reader, between readers,
  after load, during pure compilation, and before the first side effect.
- Add direct-procedure tests for wrong job, attempt, generation, count, order,
  duplicate key, unknown version, size, and digest.
- Add an exhaustive family-presence test proving every accepted immutable policy
  and target table has exactly one bounded reader and participates in identity.
- An accepted implementation is not complete until focused database/runtime
  tests, `just ci`, and `just ui-e2e` pass.

## Follow-up

- Obtain explicit operator approval before implementation or changing this ADR
  to `Accepted`.
- Decide ADRs 518, 519, 521, 523, and 524 before freezing the first aggregate
  version; decide any proposed target extensions before adding their families.
- Implement the database snapshot writer and reader contract before allowing
  the application compiler or worker to consume it.

## Task Record

- Motivation:
  - Complete the ABI-level decision intentionally deferred by accepted ADR 446.
- Design notes:
  - Transport completeness, semantic compilation, resolved roots, and capability
    evidence remain separate typed boundaries.
- Test coverage summary:
  - Proposal only; no schema, SQL, Rust, API, UI, or runtime tests were added.
- Observability updates:
  - A future implementation may count bounded error codes and family enums. Job,
    attempt, claim, path, digest, and value data must not be metric labels.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, accepted ADRs 446-451, 484, 500, and 501,
    and proposed ADRs 507-516. This record claims no implementation.
- Risk & rollback plan:
  - The proposal can be rolled back by deleting this file and its catalogue
    entries. A later implementation must reject unsupported queued snapshot
    versions rather than reinterpret or partially load them.
- Dependency rationale:
  - No dependency is proposed; the repository already uses PostgreSQL
    `pgcrypto` and Rust SHA-256 support.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and
    `.github/instructions/revaer-data.instructions.md`; no policy drift was
    found and no quality criterion is relaxed.
