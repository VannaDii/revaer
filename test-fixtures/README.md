# Media Conversion Test Fixtures

This directory defines the media conversion integration fixture suite. Media
binaries are intentionally excluded from git; only this documentation, the
manifest, probe snapshots, scripts, and tests are committed.

## Local Setup

Prepare and verify fixtures with:

```bash
just download-test-fixtures
just generate-test-fixtures
just verify-test-fixtures
just test-media-conversion
```

`just test-media-conversion` expects downloaded and generated fixture binaries to
already exist. It verifies the manifest and then runs the ignored fixture-backed
Rust integration test.

## CI Setup

CI should restore a cache for these ignored directories when available:

- `test-fixtures/source/`
- `test-fixtures/matroska/`
- `test-fixtures/chromium/`
- `test-fixtures/derived/`

When the cache is cold, CI must run:

```bash
just download-test-fixtures
just generate-test-fixtures
just verify-test-fixtures
just test-media-conversion
```

The cache key should include `test-fixtures/manifest.json`,
`scripts/test-fixtures/*.sh`, and media tool versions so stale binaries cannot
mask manifest or generation changes.

## Committed vs Ignored Files

Committed files:

- `test-fixtures/README.md`
- `test-fixtures/ATTRIBUTION.md`
- `test-fixtures/manifest.json`
- `test-fixtures/probe/*.json`
- `scripts/test-fixtures/*.sh`

Ignored files:

- downloaded upstream media
- generated derived media
- temporary script workspaces

The prepared fixture set is expected to stay small enough for CI caching. Exact
size depends on upstream corpus revisions and generated outputs; run
`du -sh test-fixtures/source test-fixtures/matroska test-fixtures/chromium test-fixtures/derived`
after preparation for the local total.

## Verification

`scripts/test-fixtures/verify-fixtures.sh` preflights required tools, runs
`ffprobe`, writes normalized JSON snapshots to `test-fixtures/probe/`, validates
manifest stream counts and stable codecs, checks derived language metadata and
forced subtitle disposition, and verifies the silent-audio fixture has an audio
stream. Any mismatch exits non-zero with the fixture id and field name.

Fixture preparation tries primary upstream URLs first. Some Test-Videos entries
also declare exact Internet Archive captures of the same URLs as fallbacks in
`manifest.json`; preparation fails rather than skipping media coverage when all
documented sources for a fixture are unavailable.
