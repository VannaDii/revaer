# Media Conversion Test Fixtures

This directory defines the media conversion integration fixture suite. Media
binaries are intentionally excluded from git; only this documentation, the
manifest, immutable acquisition lock, probe snapshots, scripts, and tests are
committed.

## Local Setup

Prepare and verify fixtures with:

```bash
just download-test-fixtures
just generate-test-fixtures
just verify-test-fixtures
just test-media-conversion
```

`just test-media-conversion` expects downloaded and generated fixture binaries to
already exist. It verifies locked source integrity and reviewed probe snapshots,
then writes a Markdown report to `target/media-conversion-report.md` by default; set
`REVAER_MEDIA_CONVERSION_REPORT` to write it somewhere else.

The catalog covers common and unusual container, codec, multi-stream, subtitle,
fragmentation, timecode, and generated-audio combinations. Runtime layers may
consume the same immutable catalog for production planner and executor tests.

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

The cache key must include `test-fixtures/lock.json`,
`test-fixtures/manifest.json`, `scripts/test-fixtures/*.sh`, and media tool
versions so stale binaries cannot mask lock, manifest, or generation changes.
For GitHub Actions, use the equivalent of
`hashFiles('test-fixtures/lock.json', 'test-fixtures/manifest.json', 'scripts/test-fixtures/*.sh')`.
Every restored file is revalidated against the locked SHA-256 and exact byte
bounds before use. The PR media-conversion job appends
`target/media-conversion-report.md` to the GitHub job summary and uploads the
same file as the `media-conversion-report` artifact.

## Committed vs Ignored Files

Committed files:

- `test-fixtures/README.md`
- `test-fixtures/ATTRIBUTION.md`
- `test-fixtures/lock.json`
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

`scripts/test-fixtures/verify-fixtures.sh` revalidates every downloaded source
against `lock.json`, runs `ffprobe` into a private temporary tree, canonicalizes
the probe output, and diffs it against the reviewed snapshots. Verification is
strictly read-only and fails on any drift. Snapshot replacement is a separate,
explicit operator action:

```bash
just update-test-fixture-probes
```

`just test-media-conversion` records source, generated-fixture, and bounded
diagnostic counts in the Markdown report. Any mismatch exits non-zero with the
fixture id and field name.

Fixture preparation tries primary upstream URLs first. Some Test-Videos entries
also declare exact Internet Archive captures of the same URLs as fallbacks in
`lock.json`; preparation fails rather than skipping media coverage when all
documented sources for a fixture are unavailable. Downloads use exclusive files
inside a private temporary directory, enforce connection and total deadlines,
cap transferred and decoded bytes, and install a file only after its hash and
byte bounds pass.
