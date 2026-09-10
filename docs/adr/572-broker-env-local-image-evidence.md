# Broker environment local image evidence

- Status: Recorded
- Date: 2026-09-10
- Operator approval: Not applicable: nonarchitectural task record
- Context:
  - ADR 558 approves B1-B3 and shared G1 while holding E1 exact environment
    values, paths, bytes, digest, and package changes for evidence and separate
    operator approval. This record supplies bounded historical observations only.
  - The authorized sidecar owns ADR 572, its index/SUMMARY entries, and generated
    documentation in `work/broker-env-evidence-20260910`. The parent owns ADR 571
    and the wire parser; the parallel vector task owns ADR 573.
- Decision:
  - Record existing-image observations without selecting or enforcing new
    environment values, changing build inputs or permissions, or building an
    unapproved prototype. E1 remains pending; no architectural approval is
    recorded or inferred here.
  - No build, pull, push, deployment, remote mutation, or GitHub/browser access
    was performed. No new dependency or build recipe was introduced.
- Consequences:
  - Both existing arm64 snapshots fail the candidate HOME requirement and lack
    one candidate PATH directory. Neither proves the current release package.
  - Missing amd64 and immutable source provenance prevent cross-platform or
    current-source conclusions, even where historical version probes succeed.
- Follow-up:
  - Use the bounded, unexecuted next-probe plan below after prerequisites allow
    it; retain E1's separate exact-value/package approval gate.

## Scope And Provenance

The worktree source at investigation was
`3ac5d9822cb76a218437a53ea71fe9fa8264e363`, committed
`2026-09-10T01:19:38-07:00`. Inventory was captured at
`2026-09-10T08:27:23Z`. Raw evidence and the executed observation harness are
outside the worktree at
`/Users/vanna/Source/revaer-reviews/2026-09-10/broker-env-572`.

Inventory was performed before any container probe, using local `docker image ls
--all --no-trunc --digests` with three separately recorded filters:
`reference=*revaer*`, `label=org.opencontainers.image.title=Revaer`, and
`label=org.opencontainers.image.source=https://github.com/VannaDii/Revaer`.
The latter filters included a relevant untagged image. Unrelated images were not
inventoried. This bounded inventory does not claim discovery of unlabeled,
untagged image blobs or artifacts outside those filters.

| Snapshot | Existing local image ID | Tags | Platform | Created, UTC | Revision label |
| --- | --- | --- | --- | --- | --- |
| Tagged | `sha256:da350429de6619c4172e0d3a7be5173fb9a0ee904028b0d9f1408924749631a7` | `revaer:latest`, `revaer:pr73-local` | `linux/arm64` | `2026-08-05T00:30:42.805075131Z` | `main` |
| Untagged | `sha256:1dd75174352bf90df28c6af9c4ada786b77b01d75c261ab775102a4d45911e86` | None | `linux/arm64` | `2026-08-15T23:56:50.860583547Z` | `492test` |

The respective image-creation ages were 36.331 and 25.355 days. Exact
source-revision ages are unknown: neither label identifies an immutable commit,
and both images have an empty `RepoDigests` array. Image IDs bind these local
observations but are not registry manifest digests, signed provenance, or a
source-revision attestation. The tag `latest` is not evidence of currency.

Explicit local `docker image inspect --platform linux/amd64` failed for each ID
with exit 1 and the daemon's platform-mismatch diagnostic; arm64 inspection
succeeded. No relevant amd64 image was found in scope, and no image was pulled,
rebuilt, retagged, or emulated as a substitute. Both snapshots report Alpine
3.23.5. The host daemon reports Docker Desktop 29.7.2, LinuxKit
`7.0.12-linuxkit`, architecture `aarch64`. This is not production containment
evidence.

## Isolated Probe Boundary

Each of 20 probes used one disposable container with the immutable local image
ID, `--pull=never`, `--platform=linux/arm64`, `--read-only`, `--network=none`,
`--cap-drop=ALL`, `--security-opt=no-new-privileges`, no healthcheck, no published
ports, no host bind mounts, and the image's unchanged `revaer` user. Resource
limits were 64 PIDs, 256 MiB, one CPU, and a 30-second bound per CLI operation.
The image's declared `/data` and `/config` volumes were covered by empty
read-only tmpfs mounts, preventing writable anonymous volumes; no media or
secrets were provided. The application entrypoint was never run. Docker's socket
was used only by the host CLI, never exposed inside a probe.

Container configuration was inspected before start. Exit state, stdout/stderr,
removal with volumes, and absence after each probe were retained. Every probe
was removed before the next one, with no timeouts or OOM events. No permission
changes, file-write trials, media operations, broker startup, or production
validator execution occurred.

`/usr/bin/env -i` supplied exactly the unchanged nine E1 candidate entries. The
dedicated environment probe returned precisely the 227 candidate bytes with
SHA-256 `03eb399aa642dcaf17de8a6329d45be135c17277add98703d700d67fae77b2b1`.
This is only a check of isolated probe input, not installation or approval of a
manifest. Version probes invoked tools directly after `env -i`, without a shell;
the filesystem inspection shell could add its own bookkeeping variables. Native
version flags were used, not an assumed universal `--version` spelling.

The candidate is deliberately observed outside production admission even though
its filesystem requirements fail. This does not establish a valid injected
native-development source or authorize bypassing those requirements in runtime.

## Candidate Filesystem Findings

All following ownership, mode, and readability observations are from the service
identity UID 100, GID 101. Modes are octal. Read/search tests succeeded for the
listed existing directories. `test -w` returned true for HOME and `/tmp`, but
these are permission observations, not successful writes through the read-only
mount. No such writes were attempted.

| Candidate key or path | Observed in both arm64 snapshots unless noted | Evidence limit or consequence |
| --- | --- | --- |
| `PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin` | `/usr/local/sbin` missing; remaining five directories UID/GID `0:0`, mode `0755`, readable/searchable | An absent candidate search directory fails the existing-directory requirement; successful PATH lookup does not resolve it. |
| `LD_LIBRARY_PATH=/usr/local/lib` | Directory `0:0`, `0755`, readable/searchable; tagged contains a `perl5` directory, untagged is empty | This is not complete dynamic-library closure or ADR 519 identity evidence. |
| `SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt` | Regular file `0:0`, `0644`, readable, 179,359 bytes, 119 PEM certificate blocks | SHA-256 `b8d837841b88bfaa1a0fa827cbca8e2576418dd47c9fc4bb7f1f9d89c83111b9`; actual trust/TLS behavior was not exercised. |
| `SSL_CERT_DIR=/etc/ssl/certs` | Directory `0:0`, `0755`, readable/searchable; `/etc` and `/etc/ssl` likewise | No full certificate-directory member audit or TLS search test. `/etc/ssl/cert.pem` is a symlink to the candidate bundle, not the candidate itself. |
| `LANG=C`, `LC_ALL=C` | Exact bytes present in both environment probes; tested commands produced their retained output | No exhaustive locale-dependent media behavior tested. |
| `TZ=UTC` | Exact bytes present; `/etc/localtime` absent in both. Tagged has `/usr/share/zoneinfo/UTC`, regular `0:0`, `0644`; untagged lacks it | No timezone fallback, timestamp-conversion, or package requirement is inferred. |
| `HOME=/home/revaer` | Directory UID/GID `100:101`, mode `02755`, readable/searchable; passwd home matches; parent `/home` is `0:0`, `0755` | Fails the root-owned, non-service-writable directory requirement. Read-only probing does not repair the underlying image metadata or exempt HOME. |
| `TMPDIR=/tmp` | Directory `0:0`, mode `01777`, readable/searchable | Matches the observed sticky-mode requirement, but temporary-file creation and cleanup were not exercised on the read-only root. |
| Candidate `/app/config/media-process-broker-env-v1` | Absent in both; `/app` and `/app/config` are `0:0`, `0755` tagged and `0555` untagged | No packaged manifest, no-follow open, immutable byte/digest proof, or runtime admission proof exists here. |

Additional observations retained in the raw output: `/lib/ld-musl-aarch64.so.1`
is a regular `0:0`, `0755` file; neither image has the checked
`/etc/ld-musl-aarch64.path` or `/etc/ld-musl-x86_64.path`. No loader-default policy
or exact closure is inferred from absent configuration files.

## Package And Version Observations

Installed-package inventories came from `apk info -v` inside the read-only,
network-disabled containers. Both returned exit 0 but emitted two warnings about
missing local Alpine repository-index cache files. Those warnings remain in the
raw stderr; no network/index refresh was attempted, and this is not a clean
compliance, vulnerability, or package-verification gate. The tagged snapshot's
reported direct media package versions match the corresponding current
`.github/build-inputs.env` entries; version agreement is not source provenance
or current-release proof. The untagged snapshot lacks the media packages below.

| Direct candidate-environment command | Tagged snapshot | Untagged snapshot |
| --- | --- | --- |
| `ffmpeg -version` | 8.0.1; exit 0 | Not found; exit 127 |
| `ffprobe -version` | 8.0.1; exit 0 | Not found; exit 127 |
| `exiftool -ver` | 13.55; exit 0 | Not found; exit 127 |
| `mediainfo --Version` | MediaInfoLib 25.09; exit 0 | Not found; exit 127 |
| `mkvmerge --version` | 96.0; exit 0 | Not found; exit 127 |
| `mkvextract --version` | 96.0; exit 0 | Not found; exit 127 |
| `curl --version` | 8.20.0; exit 0 | 8.20.0; exit 0 |
| `openssl version` | 3.5.7; exit 0 | 3.5.7; exit 0 |

The tagged media executables resolve under `/usr/bin`, are root-owned regular
files with mode `0755`, except ExifTool at `0555`. `mp4info` and Perl were also
located and statted in that snapshot but not version-executed; neither was found
in the untagged snapshot. This bounded set is not an exhaustive native-tool or
execution-closure inventory. Successful version output proves only that command
started in that historical isolated environment, not media processing, TLS,
scratch-space behavior, current broker integration, or production readiness.

## Gaps And Proposed Next Probe

`just image-build-verify` was not invoked. The inspected devops policy and
`.github/workflows/pr.yml` require successful UI shards, feature/native/media
checks, coverage/Sonar, supply-chain checks, and matrix loading before PR image
verification. The parent reported a known current-UI media failure; this sidecar
did not independently rerun it and has no passing current prerequisite bundle.
Directly invoking the recipe or another build path would not establish those
prerequisites. No replacement recipe, build-input edit, gate bypass, package
validation claim, or permission repair was made.

The following plan is proposed only and was not executed:

1. Have the parent repair and rerun the existing applicable prerequisites for
   the exact intended revision, retaining their evidence. Do not relax gates.
2. Once permitted, obtain exact-revision `linux/amd64` and `linux/arm64` package
   evidence through the existing canonical image verification path without
   introducing E1 package changes or an alternate prototype/build recipe.
3. Repeat the same read-only, network-disabled candidate filesystem and native
   version observations on both immutable IDs; bind them to the actual source
   SHA and complete package inventory. Document remaining certificate, locale,
   timezone, temporary-file and execution-closure gaps separately.
4. Present an exact-value/package proposal resolving HOME ownership and the
   absent PATH directory for separate operator approval. This record selects no
   replacement path, directory-creation step, ownership change, or value. Do not
   change or exempt any existing security check. E1 remains pending until the
   operator explicitly approves the resulting exact proposal.

## Task Record

- Motivation:
  - Replace E1 filesystem assumptions with bounded observations while the parent
    implements the independently approved wire-parser work.
- Design notes:
  - Documentation-only sidecar; historical OCI/package evidence is kept separate
    from source inspection, candidate input validation, and production proof.
  - Retained artifacts include `inventory-summary.json`, `candidate.json`, both
    platform-inspection outcomes, per-probe configuration/output/exit/cleanup
    JSON, `verification.json`, and `SHA256SUMS` under the external evidence path.
- Test coverage summary:
  - Twenty isolated probes and evidence consistency checks completed: fourteen
    probe containers exited 0 and six missing-media-tool probes exited 127 as
    recorded. All containers exited without OOM/timeouts and were removed.
  - `just docs-index` passed without warnings and generated 512 entries.
    `git diff --check` passed; only the assigned record, navigation entries, and
    generated documentation changed. Investigation stopped after the two local
    snapshots and twenty probes; no broader or replacement build was attempted.
  - Full `just ci` and `just ui-e2e` remain parent-owned by the explicit sidecar
    assignment. No project-wide gate pass, PR validation, or release readiness
    is claimed here.
- Observability updates:
  - No runtime telemetry changes. Local evidence preserves command arguments,
    timestamps, image IDs, platform failures, warnings, modes, and cleanup.
- Status-doc validation:
  - Reviewed the ADR 558 E1 hold and Dockerfile/build-input source. ADR 558 and
    production/status surfaces remain unchanged; only ADR 572, its index and
    SUMMARY entries, and generated documentation are in scope.
- Risk & rollback plan:
  - Primary risk is mistaking historical/version-only results for approval or
    current-release evidence. The provenance and gaps above are mandatory limits.
  - Revert this documentation-only commit to withdraw the record. No runtime
    rollback, permission restoration, image deletion, or media cleanup is needed.
- Dependency rationale:
  - None added. External observation/consistency harnesses use existing Docker
    CLI and Ruby standard libraries; they are not package/build inputs.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/devops.instructions.md`, the ADR
    template, ADR 558 E1, `justfile`, `just/docs.just`, `just/images.just`,
    `scripts/image-release.sh`, `.github/workflows/pr.yml`, `Dockerfile`, and
    `.github/build-inputs.env`. No policy contradiction was removed or relaxed.
  - Historical evidence identifies HOME and PATH incompatibilities, not permission
    to update package policy. The parent-owned full-gate boundary remains explicit.
