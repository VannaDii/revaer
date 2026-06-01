# Media deterministic stream ranking slice 2

- Status: Accepted
- Date: 2026-05-31
- Context:
  - `MEDIA_TRANSCODING.md` slice 2 requires deterministic stream ranking with language priority, role priority, codec quality, and stable tie behavior.
  - ADR 365 expanded semantic role inference, but callers still lacked a single deterministic ordering helper for stream sets.
- Decision:
  - Add `rank_streams` to the media core classifier.
  - Sort by stream kind, caller-provided language priority, inferred semantic role, codec quality, and stable stream id tie-breaker.
  - Encode the documented audio quality order and subtitle quality order directly in pure ranking weights.
- Consequences:
  - Positive outcomes:
    - Planning code can consume a deterministic stream order without duplicating ranking heuristics.
    - Ranking ties are stable and independent of source container order.
    - Language priority remains caller-configurable without adding dependencies or persistence state.
  - Risks or trade-offs:
    - Rank weights are still first-release heuristics; future policy profiles may need persisted configurable weights.
- Follow-up:
  - Move rank weights into compiled media profiles once target/policy schema slices are expanded.
  - Add bitrate, channel layout, placement preference, and compatibility-impact weights.

## Task Record

- Motivation:
  - Continue slice 2 by adding the deterministic ranking primitive called out in the classification plan.
- Design notes:
  - Returned cloned `MediaStream` values so ranking does not mutate the caller's source graph.
  - Used stream id as the final tie-breaker so identical streams remain deterministic.
  - Kept codec quality weights small and explicit for the documented audio/subtitle examples.
- Test coverage summary:
  - Added failing tests for preferred-language audio codec ranking, subtitle role-before-codec ranking, and stream-id tie breaks.
  - Ran the targeted ranking tests and the full media-core test package with single-threaded execution after implementation.
- Observability updates:
  - None. This is pure domain ranking logic.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: early hard-coded weights may not match every policy. Roll back by reverting `rank_streams`, its tests, and this ADR.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
