# Media semantic role inference slice 2

- Status: Accepted
- Date: 2026-05-31
- Context:
  - `MEDIA_TRANSCODING.md` slice 2 requires deterministic semantic role inference coverage for commentary, descriptive audio, forced subtitles, SDH, signs/songs, karaoke, and unknown streams.
  - The existing role inference covered commentary, forced subtitles, descriptive audio, SDH, primary, and unknown, but did not distinguish signs/songs or karaoke subtitles.
- Decision:
  - Add `SignsSongs` and `Karaoke` semantic roles to the media core classifier.
  - Normalize title matching across hyphens, underscores, and ampersands before role checks.
  - Keep forced subtitles highest priority, then commentary/descriptive/SDH/signs-songs/karaoke, then default primary, then unknown.
- Consequences:
  - Positive outcomes:
    - Anime and music-heavy subtitle tracks can be classified separately from generic subtitles.
    - SDH and descriptive-audio tests now document common title forms instead of relying only on dispositions.
    - Role inference remains pure and deterministic.
  - Risks or trade-offs:
    - The classifier still uses conservative title markers; richer language-aware role ranking remains a later slice.
- Follow-up:
  - Add deterministic stream ranking over semantic roles, language preference, default/forced disposition, and stable stream id tie-breakers.

## Task Record

- Motivation:
  - Continue slice 2 by closing missing semantic role coverage called out in the implementation plan.
- Design notes:
  - Kept classification title-based and dependency-free.
  - Preserved existing role priority so forced subtitles remain forced even when their titles contain other markers.
- Test coverage summary:
  - Added failing tests for descriptive-audio title matching, SDH hearing-impaired titles, signs/songs subtitles, karaoke subtitles, and unknown subtitles.
  - Ran the focused media-core classification test filter after implementation.
- Observability updates:
  - None. This is pure domain classification logic.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: future ranking code may need more granular role ordering. Roll back by reverting the classifier variants/tests and this ADR.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
