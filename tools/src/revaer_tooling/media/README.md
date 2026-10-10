# Media fixture tooling

`rv` acquires locked source files, generates the declared derivatives with FFmpeg,
and compares ffprobe output with reviewed snapshots. It reads the selected
checkout's [manifest](../../../../test-fixtures/manifest.json) and
[source lock](../../../../test-fixtures/lock.json). The foundation and media stack
can therefore use different inventories with the same implementation.

## Commands

Run the preparation steps in order:

```console
rv download-test-fixtures
rv generate-test-fixtures
rv verify-test-fixtures
rv test-media-conversion
```

| Command | Result |
| --- | --- |
| `download-test-fixtures` | Validate cached bytes or acquire each locked source through curl. |
| `generate-test-fixtures` | Run only the generation recipes selected by the manifest. |
| `verify-test-fixtures` | Verify all source hashes and snapshots, then publish a fresh preparation report. |
| `test-media-conversion` | Prepare fixtures and run the media crate's ignored integration suite; require positive audio/video actions and zero failures. |
| `update-test-fixture-probes` | Explicitly replace ordinary snapshots after every probe succeeds; the ADR 578 F1 snapshot is always compared and never replaced. |
| `test-media-root-catalog` | Run the media runtime's `root_catalog` tests. |
| `test-media-root-contract` | Run the API model's `media_root_contract` tests. |
| `test-media-broker-codec` | Run the media runtime's `process::broker` tests. |
| `test-fixture-scripts` | Run the Python fixture-tooling regressions, including native curl, FFmpeg and Cargo checks. |
| `fixture-cache-key` | Emit native tool and fixture implementation hashes for workflow caches. |
| `clean-test-fixtures` | Remove this checkout's acquired and derived media files. |
| `clean-test-media` | Remove temporary conversion data owned by this checkout. |

The foundation predates `revaer-media-runtime`. There, `test-media-conversion`
preserves the existing preparation-only behavior and says that it verified
fixtures. Cargo's actual workspace metadata determines whether the integration
crate exists. A Cargo metadata failure stops the command.

## Architecture

| Module | Responsibility |
| --- | --- |
| `settings.py` | Immutable settings populated at the CLI boundary. |
| `model.py` / `catalog.py` | Parse manifests, join source identities, and reject duplicate, linked or out-of-checkout paths. |
| `acquire.py` | Verify cache and staged bytes before atomic publication. |
| `generate.py` | Select declared recipes and publish complete generated files. |
| `probes.py` | Preserve native diagnostics, compare canonical streams, and defer snapshot updates until all checks pass. |
| `diagnostics.py` | Enforce the media stack's exact, existing ADR 578 F1 exception. |
| [`external/curl.py`](../external/curl.py) | Typed HTTPS download bounds, native retries, redirects and certificate verification. |
| [`external/media.py`](../external/media.py) | Typed FFmpeg recipes, ffprobe stream/duration operations and full version reports. |
| [`tasks/media.py`](../tasks/media.py) | Static command entry points, operation locking, fresh reports and owned cleanup. |

Python uses the standard library for JSON, hashing, base64, filesystem operations
and comparison. No new Python dependency is needed. curl and FFmpeg retain their
existing native roles. Their supported interfaces are documented by
[curl](https://curl.se/docs/manpage.html) and
[ffprobe](https://ffmpeg.org/ffprobe.html).

## Acquisition and generation

Every source has a SHA-256, positive minimum/maximum byte counts and ordered
HTTPS URLs. Invalid cached content is reported and removed. A valid cache is
reused unless a forced download is requested. curl keeps HTTPS restrictions on
redirects, system certificate verification, bounded transfer sizes, three retries,
and the existing connection and transfer timeouts. Personal curl configuration
cannot add transfers or override these restrictions. Normal proxy and certificate
environment settings remain available.

Each attempted source gets a separate private staging file. Raw and base64
payloads must satisfy the decoded size and hash before publication. A failed
forced refresh preserves the previous valid file and returns failure.

Generation supports the eight existing media recipes: multiple audio tracks,
subtitles, video-only MP4, audio-only M4A, silent audio, MPEG-TS, MOV and AVI.
The locked Big Buck Bunny input supplies the duration. Language, title, stream
selection and codec arguments remain explicit. Matroska subtitle generation
preserves the foundation's `default_mode=passthrough` fix, keeping the reviewed
default/forced dispositions. Existing nonempty derivatives are reused; later
snapshot verification detects incompatible content.

## Diagnostics and evidence

The process runner reads binary pipes and decodes strict UTF-8 without changing
CR, CRLF or NUL bytes. This matters because a diagnostic's framing is part of its
approval. Every probe requires a successful native exit and at most 4096 bytes
of stderr. Ordinary diagnostics are allowed only for the manifest's reviewed
Chromium fragmented MP4 fixture.

ADR 578 F1 applies only to `mkv-theora-vorbis-live-style`, at its exact upstream
revision, source path, source hash/size and reviewed snapshot hash. One exact
LF-terminated diagnostic, including its bounded lowercase hexadecimal address,
must match. The full version report must match one of the two approved hashes.
Any identity, snapshot, diagnostic or emitting tool-profile drift fails. Empty
stderr needs no exception. Removing the contract restores strict diagnostics.

Raw diagnostics and generated snapshots survive under
`REPORT.probe-evidence/probe-*`. Accepted F1 emissions additionally retain their
exact stderr, complete version report and classification under
`REPORT.probe-evidence/adr578-f1.*`. Evidence directories are private and files
use mode 0600. An incomplete or changed evidence write cannot establish success.

Verification removes an old report before checking the current inputs. The
conversion task publishes preparation evidence separately, runs the actual
ignored Cargo integration test with all features, and requires a newly written
report with positive pipeline, video and audio counts and zero failures.

## Configuration and ownership

| Environment variable | Default |
| --- | --- |
| `REVAER_FIXTURE_LOCK_PATH` | `test-fixtures/lock.json` |
| `REVAER_FIXTURE_MANIFEST_PATH` | `test-fixtures/manifest.json` |
| `REVAER_FIXTURE_PROBE_DIR` | `test-fixtures/probe` |
| `REVAER_MEDIA_CONVERSION_REPORT` | `target/media-conversion-report.md` |
| `REVAER_FIXTURE_FORCE_DOWNLOAD` / `REVAER_FIXTURE_FORCE_GENERATE` | `0`; use `1` to refresh. |
| `REVAER_FIXTURE_CONNECT_TIMEOUT_SECONDS` | `15` |
| `REVAER_FIXTURE_DEADLINE_SECONDS` | `120` |
| `REVAER_FIXTURE_CURL_BIN` / `REVAER_FIXTURE_FFPROBE_BIN` | `curl` / `ffprobe` |
| `REVAER_FIXTURE_FFPROBE_CONTAINER` | Optional caller-provisioned approved ffprobe container; mount the selected checkout read-only at `/workspace`. |

Paths can be absolute or checkout-relative; both must remain inside the selected
checkout. Paths containing `..` or linked components fail. Acquisition,
generation, verification and cleanup share a checkout-local operation lock.
Acquisition/generation/cleanup refuse tracked destinations. Explicit snapshot
updates intentionally edit the declared reviewed files.

Conversion subprocesses receive `TMPDIR=target/media-conversion/tmp` as an
absolute path. Cleanup does not search global temporary directories or other
worktrees. It preserves manifests, reviewed snapshots and reports. Native tools
must be installed through their package manager; Linux automation can select
`rv setup --profile python --no-launcher --apt-profile media`.

## Validation scope

The focused suite covers corrupt caches, fallback sources, native HTTPS and
redirect failures, malformed manifests, exact diagnostic mutations, missing
evidence, report freshness and cleanup ownership. Actual FFmpeg generated all
eight derivatives and matched the reviewed snapshots. A small Rust 2024 fixture
proves ignored integration selection, all features, report checks and owned
temporary paths. It does not prove the application's conversion engine.

The full recorded media inventory also passed through the actual `rv` CLI in a
disposable checkout: 22 locked sources, eight generated files and 30 reviewed
snapshots. The real F1 emission matched the approved Homebrew 9.0.1 profile and
retained its evidence. See the [task ADR](../../../../docs/adr/592-python-tooling.md)
for logs and the remaining application acceptance work.
