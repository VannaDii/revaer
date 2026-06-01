# Media codec and language alias normalization slice 2

- Status: Accepted
- Date: 2026-05-31
- Context:
  - `MEDIA_TRANSCODING.md` explicitly calls out codec aliases such as `dca -> dts` and `subrip -> srt`, plus regional language tags such as `eng-US -> eng`.
  - The existing normalizer handled common H.264/H.265 and short English/French/German aliases, but left these listed tool-output forms unchanged.
- Decision:
  - Normalize `dca` audio codec output to `dts`.
  - Normalize `subrip` subtitle codec output to `srt`.
  - Strip language region suffixes separated by `-` or `_` before mapping language aliases.
- Consequences:
  - Positive outcomes:
    - Diffing and planning see canonical DTS and SRT values regardless of ffprobe naming differences.
    - Regional language tags collapse to the existing canonical ISO-639-3 keys before ranking.
  - Risks or trade-offs:
    - Only the listed first-release aliases are covered; broader language and codec maps still need expansion as policies grow.
- Follow-up:
  - Add broader subtitle, audio, and language alias tables when target/policy profile compilation needs them.

## Task Record

- Motivation:
  - Continue slice 2 by closing explicit normalization examples in the media-transcoding plan.
- Design notes:
  - Extended the existing pure normalizer rather than introducing a lookup dependency.
  - Kept region stripping deterministic and limited to common separator characters.
- Test coverage summary:
  - Added failing tests for `dca`, `subrip`, and `eng-US` normalization.
  - Ran the focused normalization test filter and media-core Clippy pass after implementation.
- Observability updates:
  - None. This is pure normalization logic.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: future policies may need more nuanced regional language retention. Roll back by reverting the normalizer aliases/tests and this ADR.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
