# Media conversion fixture plan

- Status: Accepted
- Date: 2026-06-11
- Context:
  - `MEDIA_TRANSCODING.md` is the executable plan for the media transcoding
    subsystem.
  - The plan needed a reproducible fixture-backed testing path for real media
    conversion behavior across containers, codecs, subtitles, stream selection,
    silent audio, video-only input, audio-only input, and edge-case MP4/MKV/WebM
    files.
  - Repository policy requires downloaded or generated media binaries to stay
    out of source control unless they are already present, while task records
    remain tracked through ADRs.
- Decision:
  - Add a dedicated media conversion fixture-suite section to
    `MEDIA_TRANSCODING.md`.
  - Require committed fixture metadata, attribution/license notes, manifests,
    scripts, normalized probe snapshots, and tests while keeping downloaded and
    generated media binaries ignored.
  - Specify deterministic source fixture names from Test-Videos Big Buck Bunny,
    the official Matroska corpus, and Chromium media test data.
  - Specify derived FFmpeg-generated fixtures for multi-audio, subtitle, silent
    audio, video-only, audio-only, MPEG-TS, MOV, and AVI coverage.
  - Require Justfile-backed preparation, verification, cleanup, and media
    conversion integration-test commands.
  - Require CI cache and execution wiring that can prepare fixtures without
    committing media binaries.
- Consequences:
  - Positive outcomes:
    - The media plan now has an explicit, reproducible integration fixture
      strategy before future implementation slices add tests.
    - Future work has concrete fixture ids, paths, sources, script names,
      manifest fields, verification rules, and acceptance coverage.
    - License and attribution evidence is part of the fixture contract instead
      of an afterthought.
  - Risks or trade-offs:
    - CI fixture preparation will rely on external sources when the cache misses.
    - Fixture-backed tests can increase runtime and disk use compared with pure
      unit tests.
    - The future implementation must normalize probe snapshots carefully so
      unstable FFmpeg fields do not create noisy diffs.
- Follow-up:
  - Implement the fixture tree, scripts, manifest, attribution notes, Justfile
    recipes, CI cache wiring, and media conversion tests in the media fixture
    implementation slice.
  - Verify upstream license evidence while implementing the downloader before
    accepting each fixture source.

## Task Record

- Motivation:
  - Make media conversion test coverage concrete enough for a new contributor
    to implement reproducible fixture-backed tests without committing media
    binaries.
- Design notes:
  - The plan keeps source, Matroska, Chromium, and derived fixture binaries in
    ignored directories while committing metadata and verification artifacts.
  - Verification is based on `ffprobe` JSON and production pipeline assertions
    rather than FFmpeg log parsing.
  - The task surface follows existing Justfile conventions with kebab-case
    recipe names.
  - The fixture acceptance matrix covers the containers, codecs, stream shapes,
    subtitle behavior, and explicit failure contracts required by the media
    conversion pipeline.
- Test coverage summary:
  - This change is documentation-only and adds no executable tests yet.
  - Verification for this task is the repository documentation and policy gate
    run recorded before handoff.
- Observability updates:
  - No runtime observability changes were made.
  - The future fixture verification script is required to print clear per-fixture
    mismatch summaries.
- Status-doc validation:
  - Updated `MEDIA_TRANSCODING.md` with the fixture suite, implementation-slice
    entry, and verification-gate requirements.
  - Updated `docs/adr/index.md` and `docs/SUMMARY.md` to reference this ADR.
- Risk and rollback plan:
  - Risk: future implementers may treat fixture preparation as optional if CI
    wiring is not added with the implementation slice. Mitigation: the plan now
    names the dedicated Justfile and CI requirements.
  - Risk: upstream fixture sources may change or become unavailable. Mitigation:
    the plan requires committed manifests, attribution evidence, normalized
    probe snapshots, clear download errors, and cacheable binary directories.
  - Rollback: revert this ADR plus the `MEDIA_TRANSCODING.md`,
    `docs/adr/index.md`, and `docs/SUMMARY.md` edits.
- Dependency rationale:
  - No new dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`,
    `.github/instructions/rust.instructions.md`,
    `.github/instructions/revaer-data.instructions.md`, and
    `.github/instructions/devops.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
