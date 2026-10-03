# Python development and automation tooling

- Status: Accepted; implementation in progress
- Date: 2026-09-14

## Motivation

Consolidate authored development and CI tooling in Python and replace Just and Node-based release/test tooling. The user approved the architecture and execution in the task conversation.

## Design notes

The `rv` CLI maps command strings to static `Task.run(context)` methods. Typed
external-tool classes verify executables and construct argument arrays through an
injected process runner. Settings and command options are immutable; environment
reading and concrete collaborator construction belong at the CLI boundary.
Internal task composition uses direct calls and does not require public commands.
The [tooling README](../../tools/README.md) describes the architecture and the
[migration inventory](../../tools/migration.md) tracks recipe parity.

### Official package-manager ownership

The root `setup.sh` uses uv's official versioned installer when uv is absent,
then runs `uv sync --locked` and `uv run --locked -- rv setup`. uv owns Python
downloads, environments, dependency resolution, packaging, and executable
installation. There is no wheel extraction, platform download table, manually
populated environment, or copied launcher executable. An incompatible existing
uv installation is reported; its package manager remains responsible for upgrades.

uv dependency groups keep the complete local/CI environment as the default while
allowing container stages to install the core CLI with `--no-default-groups`.
YAML and JSON Schema remain core dependencies; development checks, browser tests,
and the version engine use the `dev`, `testing`, and `release` groups. The official
uv commands own this selection, and audits export all groups. An actual official
uv/Alpine container downloaded managed CPython 3.13.12 for musl and ran the installed
CLI with an unchanged lockfile and no browser/release packages or warning lines.

`uv tool install` installs a small dependency-free launcher snapshot. That launcher
finds the current Git/worktree boundary and invokes its locked `rv` implementation.
It is independent of the checkout from which it was installed. Supported uv
installer, cache, index, and tool-directory settings remain available; inherited
Python/project overrides that could select another worktree are cleared.

Rustup and Cargo own native installations and receipts. Setup does not force
matching Cargo tools to rebuild. The media stack's exact cargo-udeps/nightly pins
are recorded in `tools/versions.toml`; analysis rejects conflicting overrides and
retains compiler evidence. Setup orchestration tests inject the installer boundary
instead of modifying global native installations.

### Release and test behavior

The setup composite now uses the official `astral-sh/setup-uv` action, the root
version constraint and lockfile, then calls the same typed setup task as local
development. Cargo/browser selections and native apt profiles are explicit.
An actual official uv/Debian container passed first installation, repeat setup,
invalid package input, missing-package failure, recovery and unchanged-lock checks.
Workflow mutation tests also parse the committed action's CLI arguments.

Documentation and manual Helm workflows now invoke Python tasks. The docs path
preserves source/type guardrails, complete book/LLM payloads, optional CNAME and
the existing gh-pages publisher. Real repository validation exposed orphan links
to ADRs 378/379 in shared prerequisite `ec8adc14`; those ADR files are absent from
this foundation. Remove those orphan index entries here and discard only the
empty files mdBook generated. The integrated media checkout contains the actual
records, so its combined documentation index must retain them during integration.
The docs task verifies the pinned mdBook/mermaid pair before building, preventing
a different PATH installation from emitting a protocol-version warning.

Manual and reusable chart version selection shares one task and the native gh
API interface. Real local API tests verify encoded query values, PR/run-number
defaults, manual overrides, ambiguous matches and transport failures. The same
SemVer validator is used by metadata resolution and chart packaging. GitHub
outputs and environment files use a shared, validated multiline writer.

The reusable image workflow now invokes the same Python stages as local tooling.
It retains authorization conditions, bounded architecture jobs, the independent
finding gate, failure-time SARIF upload and downstream manifest/chart ordering.
Buildx metadata must match an independent registry resolution before completion.
Manifest inputs resolve to immutable digests before creation; all destination tags
must agree before one digest is signed. Cosign retains keyless signing and the
existing repository/workflow, issuer and predicate checks, with literal repository
dots. Existing chart packages are read through Helm and checked against their
requested versions before verification or signed publication.

Python Semantic Release selects versions and generates notes. `rv` coordinates
source-bound application artifacts, Helm/GPG signing, GitHub CLI asset transfers,
and explicit partial-release recovery. Expected assets must match local and remote
digests; retries upload only missing files. Completion outputs are emitted after
the whole publication verifies. The engine and asset CLI share an explicit
repository destination, and conflicting GitHub CLI overrides are rejected.

Existing stable tags retain their detached-checkout workflow through Git and
`gh release create --verify-tag`; this path neither calculates a new version nor
invents a branch eligible for the release engine. The
[release README](../../release/README.md) documents preparation and recovery.

Rust test variants preserve their package and feature sets. They carry the newer
media contract forward by requiring a caller-selected disposable database and
serializing native tests. SQLx remains responsible for migrations; checksum and
SQL failures never trigger a reset in `rv db-migrate`. Python Playwright and
pytest implement the E2E migration. The full foundation application suite and
route gate passed. Media scenario integration remains in progress.

### Database proof resources

Disposable proof data uses Docker's anonymous volume lifecycle. Docker owns
creation and exact-ID removal; a post-removal query verifies the recorded volume
is absent. This avoids host bind-mount ownership cleanup while preserving native
candidate output. Creation identity is recorded before allocation; uncertain
creation is recovered only by the exact generated name and ownership label.
Positive candidate evidence waits for successful cleanup. The data instruction
file covers these operational Python paths; application database access is unchanged.

Pristine catalog generation uses the same owned lifecycle with no published
network port. A validated immutable projection covers all 27 existing catalog
classes. The read uses a direct constrained owner; populated credential-bearing
catalogs retain permission failures. Physical exclusions and OID normalization
remain unchanged. The package README explains query construction, exact-byte
comparison, private provenance, and failure behavior. Commands publish observed
evidence only after cleanup and never rewrite the committed reference.

### Worktree and integration boundary

Database lifecycle defaults now match the active media stack's `revaer`
application database. Caller-supplied URLs remain caller-owned by default.
Managed containers use checkout-specific names, directory ownership records,
local operation locks, and a server-side identity check before migration/reset.
PostgreSQL's supported `PGDATA` option separates its data from ownership metadata;
container recreation preserves those data. SQLx owns history, create, and explicit
reset; libpq owns readiness and transactional seed execution. Migration failures
preserve data instead of triggering the old recipes' automatic reset. Existing
unlabelled containers/data are not adopted; the README documents parallel migration
with an available port and fresh data directory. No existing developer database
has been reset or replaced during validation.

Implementation worktree: `work/python-tooling`, initially based on `54641c63`.
The existing prerequisite layer ending at `ec8adc14` is reused as its shared
ancestor with the media stack. These twelve commits already supply the advisory
removals, native libtorrent 2.1 handling, and Rust 1.96 baseline. Reuse preserves
their history and avoids a second set of application dependency fixes. Original
foundation proofs below retain their original source identity; acceptance must
be rerun on the new common baseline. ADR 592 follows the active integration's
uncommitted ADRs 590 and 591 to avoid reusing their numbers.
The ongoing media integration is independently active and remains untouched.
Its `f1a8624f` snapshot (2026-09-15) includes substantial additional shell and Ruby policy,
database proof, media fixture, and compliance tooling. These need explicit task
mappings and parity checks before integration can be declared complete. Refresh
that snapshot before creating the disposable combined validation checkout.

## Test coverage summary

- After the complete database guard/probe port, the full suite passed **1422
  tests**, **95.04%** overall coverage and **94%** implementation/launcher coverage.
  Formatting (265 files), Ruff and strict mypy (247 sources) passed. Evidence:
  `artifacts/database-guards-tooling-coverage.log`. The new Python instruction
  file records the accepted architecture and official uv ownership; the stale
  Rust-scoped precedence claim was corrected to follow root policy.

- The Rust qualification entry points and existing settings/dispatch/Cargo
  regressions passed **63 tests** (`artifacts/database-probes-validation.log`).
  Native fixture crates prove all-feature baseline selection, exact library-only
  pool/cancellation names, input-file propagation, shown output and nonzero
  failures. A zero-match Cargo run now fails the qualification. The actual
  application database producer and media acceptance remain outstanding.

- The changed-line port and shared process/release regressions passed **160
  tests** (`artifacts/changed-lines-verified.log`). A real disposable Git
  repository reconstructed and deleted all 213 approved binary objects,
  verifying 21,064,113 original bytes and exact native metadata. Provider history
  is synthetic in this proof; no canonical PR exception acceptance is claimed.
  The port preserves the limits, pinned inventory, permanent expiry and exact
  checked-head requirement, replacing Ruby paths with the Python implementation
  and its dispatch/transport dependencies. Binary subprocess capture retains
  original bytes without text decoding or blob logging.

- The complete suite after the pristine port passed **1320 tests**, **94.90%**
  overall coverage and **94%** implementation/launcher coverage. Ruff and strict
  mypy passed (240 source files). Evidence:
  `artifacts/pristine-tooling-coverage.log`. This includes real command validation,
  reference mismatch and failed-read cleanup; it is not full application acceptance.

- Python pristine generation reproduced the full reviewed catalog: 8,456 lines,
  4,354,567 bytes and SHA-256
  `0ba173f3caa88da40a4391e9bd34ac88416a2f7c41f19be47043bfa54a2cbf05`.
  Evidence: `artifacts/pristine-native-parity.json`; 52 initial native mutation
  and malformed-input checks passed in `artifacts/pristine-mutation-verified.log`.
  The first run stopped on the deliberately disconnected subscription fixture's
  warning. The fixture now asserts that complete expected diagnostic; normal SQL
  operations still fail on warnings. Follow-up task checks validate exact bytes,
  retain observed evidence on mismatch and remove stale success after failed reads.
  The media integration advanced to `13082a69`; all six reviewed pristine inputs
  remain unchanged. `artifacts/media-tooling-inputs-13082a69/source-identity.json`
  records the current copied automation inputs for subsequent port comparisons.

- After adding database transition commands, the complete tooling suite passed
  **1262 tests**, **94.88%** overall coverage and **94%** implementation/launcher
  coverage. Formatting (250 files), Ruff and strict mypy (233 sources) passed;
  the 91-package locked graph passed the dependency audit. Evidence:
  `artifacts/database-tooling-coverage.log` and
  `artifacts/database-dependency-audit.log`. This remains tooling validation;
  full foundation/media application acceptance is still required.

- The native candidate proof used a disposable copy of the media inputs and left
  the active checkout untouched. SQLx 0.8.6 and the exact PostgreSQL 16.14 image
  reproduced all three existing artifact bytes: candidate SHA-256
  `fb0d371f1afe365a72bbe34e9c8fe628e318be6de0e1f43b6f73c37897c95858`,
  statement map `25ff25a6187ba982851d8f6a2c2a784f06ccb5351f4c51ffa75c49a4fa33c3a9`,
  and evidence `f4222edb62d745a9e3a3892bc40bee2c695f76d3f2818e7e8172aea221ba2a80`.
  `artifacts/database-native-proof/result.json` records the copied workspace and
  verified container/volume removal. The reviewed final initializer and generated
  security/grants for 532 routines also match the Ruby implementation;
  `artifacts/database-final-sql-parity.json` records their hashes. Failure tests
  cover creation/start/readiness, SQL errors and warnings, interruption, changed
  schemas, bounded migration retries and failed cleanup. Complete final runtime
  and ingestion proofs are still outstanding.

- One Alpine core-environment verification initially failed because its command
  used macOS's unshared system temporary directory. Re-running with the task's
  existing shared `artifacts/tmp` directory passed all 62 selected native/core
  checks. This was a validation-environment correction; no gate or source behavior
  was relaxed.

- The complete tooling coverage run passed **1137 tests**, **94.76%** overall
  coverage and **94%** implementation/launcher coverage. Ruff and strict mypy
  passed for the current implementation. Evidence:
  `artifacts/tooling-development-coverage.log`. The preceding full run caught a
  self-session guard ordering issue when pytest itself led its Unix session;
  cleanup now rejects its own PID before inspecting the session token. A real
  non-session child has a separate regression case. The earlier failed run is
  retained in `artifacts/development-tooling-check.log`.

- Development commands passed **87 focused checks**, including actual Cargo
  and Trunk serving, native Git/file events, process identity and orphan cleanup,
  failure recovery, database lifecycle and dispatch. The **32** native process
  and watcher tests also passed in a disposable Linux container using official
  uv project/group installation. Logs are `artifacts/development-final-validation.log`
  and `artifacts/development-linux-validation.log`; the container was removed.
  The complete lockfile dependency audit passed with the new development SDKs.
  These checks do not establish full Revaer application development acceptance.
- Two fixture issues and one host issue were kept as evidence: a queued macOS
  `.gitignore` notification is a valid source event; nested Cargo fixtures need
  their own workspace declaration; and the globally selected Xcode installation
  now requires license acceptance. The installed Command Line Tools passed the
  native build through per-process `DEVELOPER_DIR`. No license was accepted and
  the global developer directory was not changed. Earlier failed logs remain
  under `artifacts/development-*validation.log`.

- Media fixture/process/Cargo/dispatch checks passed **183 tests** in
  `artifacts/media-fixture-validation.log`. This includes 49 acquisition/schema
  cases, 70 diagnostic-contract cases, real native generation of all eight
  recipes against reviewed snapshots, and a small Rust 2024 ignored integration
  fixture. The subsequent URL/path and stale-catalog regressions passed in the
  complete 1089-test run recorded below. No media diagnostic approval was added
  or broadened.
- The actual `rv` CLI acquired all **22** locked media sources, generated **8**
  derivatives and matched **30** reviewed snapshots in a disposable checkout.
  The real F1 diagnostic matched the approved Homebrew 9.0.1 full report hash;
  its raw evidence was retained. `artifacts/media-fixture-proof/validation.log`,
  `result.json`, and `target/` retain source/implementation hashes and results.
  This proves fixture preparation and probing, not application conversion.
- Linux source kcov installation/reinstallation and all four measured bootstrap
  cases passed with uv's supported child-limit option. The complete log, source
  receipt and raw coverage cases are in `artifacts/kcov-linux-uv/`.
- The first complete run after the media port failed **9** release cases, with
  **1075** passing, because the GitHub adapter expected newline-normalized HTTP
  headers. Native CRLF preservation exposed that dependency. The HTTP boundary
  now accepts CRLF/LF headers without changing body bytes; **62** focused
  release/external/process checks passed, including actual PSR/GitHub CLI runs.
  Logs: `artifacts/tooling-media-coverage.log` and
  `artifacts/release-crlf-validation.log`. The failed full run is not acceptance.
- Latest complete shared-baseline tooling validation: **1089 tests passed**, **94.81%**
  overall coverage and **94%** implementation/launcher coverage. Ruff and strict
  mypy passed. Evidence: `artifacts/tooling-media-recovery-coverage.log` (2026-09-15).
  This includes the media port and native generation/Cargo fixtures, exact process
  bytes and corrected HTTP parsing, official uv child limits, native Dockerfile
  and Linux apt fixtures, dependency groups, release workflow contracts,
  documentation, compliance, and Sonar/browser/bootstrap regressions.
  Full application and combined-media acceptance are separate.
- The Python compliance generator matched the legacy metadata and all five
  artifact contents against the same controlled fixture inventory. Evidence:
  `artifacts/compliance-parity/comparison.json`. This does not establish actual
  image compliance. The real foundation docs build and deployment preparation
  passed without publishing Pages; their logs are
  `artifacts/docs-build-validation.log` and `artifacts/docs-deployment-validation.log`.
- Image coordination and workflow fixtures passed **129** focused checks. Actual
  Docker/Trivy tests passed **11** checks, including loaded architecture/source
  labels and retained failing SARIF. Chart/release/setup/image-workflow checks
  then passed **111** checks. Logs: `artifacts/image-release-focused.log`,
  `artifacts/image-native-validation.log`, `artifacts/image-workflow-contracts.log`.
  Registry publication and keyless signing are simulated in the image coordination
  fixtures; these are not hosted-workflow or full application-image acceptance.
- Container preparation passed **98** focused checks, followed by an actual
  native arm64 build of the unchanged Dockerfile with a small Rust fixture.
  uv installed managed musl Python and the locked core CLI. Runtime checks proved
  exact package versions, nonroot account/file ownership, exec-form health
  configuration, and absence of Python tooling files from the final image.
  Logs: `artifacts/container-tasks-validation.log` and
  `artifacts/container-dockerfile-proof/`. A transient Alpine DNS failure was
  retained in `artifacts/container-dns-failure.log`; the unchanged retry passed.
  Complete Revaer image acceptance remains outstanding.
- Completed Python suite including runbook, native inputs, image and registry checks: **286 tests
  passed**, **94.45%** overall line/branch coverage, and **93%** implementation and
  launcher coverage against the independent 90% floor. Ruff lint and strict mypy
  also passed. These results precede adoption of the existing media prerequisite layer.
- Real-tool checks cover uv bootstrap and multi-worktree dispatch; process I/O,
  redaction, timeouts and descendant cleanup; source/workflow policy; Cargo build,
  test feature selection and unused-dependency failures; signed chart packaging;
  and SQLx replay/checksum/transaction behavior against isolated PostgreSQL.
- Release tests run the actual PSR and GitHub CLIs against local Git remotes and
  an HTTPS/Unix-socket API fixture. The fixture rejects unexpected repositories
  and exercises creation/upload failures, conflicting assets, recovery, digest
  verification, and detached stable tags. No external release or registry upload
  was performed.
- The initial foundation passed `rv check` on Linux with libtorrent **2.0.10**, across the
  workspace, all targets and all features, with compiler warnings treated as
  errors. The shared baseline now includes the media stack's libtorrent 2.1 ABI
  support. Its macOS `rv check` and both Clippy passes subsequently passed with
  Rust 1.96.0 and libtorrent 2.1.1; the logs are
  `artifacts/shared-baseline-check.log` and `artifacts/shared-baseline-lint.log`.
- The foundation passes `rv test`, `rv test-features-min`, and `rv test-native`
  in the owned Linux/PostgreSQL validation environment. Its containers, network,
  and database data were removed after validation; source/tooling provenance and
  logs are retained in `artifacts/python-foundation-validation.json` and the
  associated attempt directory. Full `rv ci`, workflow integration, and
  combined-media validation remain pending. Fixture passes do not replace them.

## Observability updates

Media task settings, catalog validation, acquisition, generation and diagnostic
classification now have separate modules, typed native adapters and a structured
[README](../../tools/src/revaer_tooling/media/README.md). Static tasks own locking,
report publication and cleanup. The selected manifests choose inventory size.
Native curl retains transport/retry behavior; FFmpeg retains processing and codec
selection. The process runner now preserves exact native line endings, preventing
CRLF normalization from turning a rejected F1 emission into an accepted one.
Binary atomic writes retain private evidence without newline conversion.

Conversion temporary files now belong to `target/media-conversion/tmp` in the
selected checkout. Cleanup no longer searches global temporary directories by
name. An actual media crate requires a newly written integration report; a
foundation without that crate preserves its existing preparation-only behavior.
Python's standard library covers this port, so no Python dependency was added.
The existing native FFmpeg prerequisite is included in the coverage setup profile
because tooling coverage now executes its recipes.

The post-merge/tag workflow uses the same Python release tasks as local runs.
Artifact downloads bind to their actual producer SHA; no-change development
releases cannot trigger downstream chart/image publication. Stable tags use the
existing tagged release path and pass verified version outputs to chart consumers.
Chart registry jobs consume the already-signed artifact. Structured workflow
mutation tests cover these contracts; no hosted release was published.

Browser line evidence now combines grammar-derived source locations from the
official Tree-sitter Python bindings with actual Chromium V8 counters. Every
checkout JavaScript file remains represented, including unused modules and
identical vendor copies. Source hashes and UTF-16 offsets prevent attribution
to different bytes; nested zero ranges remain uncovered. Real Chromium fixtures
verify Unicode/CRLF, classic/module execution, repeated navigation, early page
closure and failed-capture artifact retention. All 74 JavaScript files in the
current working inventory parsed successfully (30 distinct byte sequences,
11,844 source-code lines); this is a source-inventory proof, not application
browser coverage acceptance.

Root bootstrap tests now run directly and through kcov against fixture-owned
checkouts. Eight cases preserve official uv installer behavior, installation
directory precedence, launcher ownership, foreign environment isolation and
stale-lock rejection. The measured cases retain source identity and raw native
records. The documented DEBUG method removes the earlier need to patch a
generated helper. uv's official `UV_RUN_RLIMIT_NOFILE` option now sets the child
descriptor limit immediately before the native command starts. The custom Python
resource-limit/exec helper was removed. Actual uv tests preserve the parent's
limits, literal arguments, and foreign environment values without creating an
environment in an unrelated checkout.

The native installer uses the existing Linux source revision and archive hash,
CMake's supported commands, a complete file/mode receipt, and staged publication.
Seven real-CMake/archive regressions passed, including failed-build preservation;
23 process-limit and report-corruption checks also passed. An actual macOS source
build failed upstream DWARF-header discovery; local kcov 43 measurement passed.
Linux source installation, repeat installation and all four measured bootstrap
cases then passed in a fresh Debian/Python container. Evidence is retained in
`artifacts/kcov-linux/validation.log`, `result.json`, and `cases/`. This establishes
the native installer and measurement path; full application acceptance is separate.
The shared Docker filesystem was full, so this validation used a temporary Colima
profile named `rv-tooling` with its own disk and explicit `colima-rv-tooling`
context. The active context remains `desktop-linux`. Remove this task-owned
validation profile after final acceptance.

The Sonar migration retains exact event SCM attribution, one scanner invocation,
the full submitted binary report, positive coverage metrics, and strict issue and
hotspot checks. API failure evidence belongs to the last attempt and cannot reuse
an earlier success. Native compilation database generation now leaves the build
script's staged CXX headers in `coverage/cxxbridge/include`; temporary JSON output
must not cause those headers to disappear. Real shared-baseline generation passed
at `artifacts/shared-baseline-native-inputs.log`.

Python analysis reports combine actual tooling and named E2E coverage databases
through Coverage.py's supported commands. One reporting source root preserves
full relative filenames, and the complete authored-file inventory retains
never-imported files with zero hits. Corrupt input data fails instead of being
skipped. The independent tooling coverage threshold remains unchanged.

The official SonarScanner 8.1.0.6389 macOS AArch64 archive passed download, pinned
SHA-256, primary signing fingerprint, extraction, and version verification, then
passed repeat installation without warnings. GnuPG uses the committed fingerprint
as the trust anchor in its private temporary home. Logs:
`artifacts/sonar-installer-verification.log` and
`artifacts/sonar-installer-repeat.log`. The first log records the diagnostic that
led to making that trust anchor explicit; the repeat log has no warning.
The Sonar developer CLI returned an organization entitlement error and snippet
analysis returned an internal error. Neither is an analysis result; full scanner
and workflow acceptance remain outstanding.

The Rust coverage replacement is verified against a real Rust/C++ fixture:
per-crate 90% Rust gates, all-feature execution, native line records, complete HTML/text
reports, threshold failure evidence, and stale-report invalidation on compiler
warnings. It uses Cargo metadata for workspace membership and retains Python
coverage files. Full foundation coverage passed in the supported Linux environment;
combined-media coverage acceptance remains pending.

The first full foundation collection passed its tests but reported 80.58% for the
combined libtorrent Rust/C++ package, including 48.60% for `session.cpp`. Review
confirmed the old gate measured Rust. The implementation now retains that Rust
criterion using unrounded engine counts, while preserving every C++ measurement
in all reports. Collection also enables the existing native integration tests.
LLVM HTML needs compiler-mapped standard-library sources, so setup includes
rustup's `rust-src` and reporting uses supported LLVM path equivalence. A fixture
verifies its response file with spaces and quotes. The original failed run is
retained under `artifacts/foundation-attempts/3038650117089313`. The corrected full
foundation run passed every per-crate gate and produced LCOV, text and HTML without
compiler/report warnings or errors. Evidence is retained under
`artifacts/foundation-attempts/05c089b59ba33f0e`, with tooling source SHA-256
`c734fe38da6fd5d6faf7cd28df5ad11c37d6f853a54f6c1cb266e79f3ed8b69b`.
No Sonar source or coverage filter was added.

The foundation Python suite collects 35 API scenarios and 13 browser scenarios.
Stateful torrent steps now form one retryable scenario. Indexer category tests
create their own catalog/profile instead of depending on collection order;
Torznab setup failures cannot bypass later assertions. Native browser fixtures
verify storage state, retry artifacts, closed-page videos, accumulated routes and
shard selection. Failure-path checks cover startup, migration, interruption,
process cleanup, failed summaries, and recording retention; all three browser
engines are exercised against fixtures. Empty shards are accepted only when the
complete collection is nonempty, and xdist workers must agree on collection.

The full foundation application run passed 35 anonymous API checks, 35 keyed API
checks, and 13 Chromium scenarios, followed by complete operation/route coverage.
Evidence: `artifacts/foundation-attempts/248eb7f6b7eb72f2`, tooling SHA-256
`172784b723f7c58761b9c0a6fdb4d440c4410bb903584dc65e0ab28f855b8aa9`.
The runbook repeats the same suite and gate while holding the E2E lock through
artifact archival. Its full application run passed at
`artifacts/foundation-attempts/36aac957c5439e40`, tooling SHA-256
`c79e83f52344b2ebaed8c4fd27399ea828cd54c4a0150084ccefa16898b1faec`.
These proofs cover Chromium application behavior; Firefox/WebKit application
runs and combined-media acceptance are still separate outstanding checks.

Strict API validation exposed drift in the hand-authored OpenAPI document. The
contract now includes the existing tested indexer operations and actual response
statuses, and models the flattened torrent detail response. The exporter only
serializes that document; it does not generate schemas from Rust types. The API
README records the update and validation procedure.

An earlier application attempt (`57db0d6be9c0a043`) recorded aborted PostgreSQL
transactions and API authentication HTTP 500 responses. The cause is unconfirmed;
no runtime database fix is claimed. A broad UI status selector was corrected to
identify the spinner, and an independent browser response guard now fails on
unexpected local HTTP 500 responses. Both subsequent full application runs passed
that guard. The original server logs remain evidence for further investigation.

Named CLI tasks, streamed redacted stdout/stderr, explicit exit status, release
preview/results, source/digest manifests, and compiler/coverage reports provide
evidence at task boundaries. Process cancellation terminates only owned process
groups. Validation logs and the foundation snapshot record live under `artifacts/`.
Temporary Docker validation data uses owned mounts or temporary storage, preserving
other worktrees and containers even when Docker's shared filesystem is full.

## Risk and rollback plan

### Release dependency acceptance blocked (2026-09-21)

A fresh `rv tooling-audit` reports three GitPython 3.1.59 advisories:
PYSEC-2026-3982, PYSEC-2026-3983 and PYSEC-2026-3984, fixed from 3.1.60.
The temporary parser-compatibility constraint is no longer acceptable for
release acceptance. Current PyPI metadata for PSR 10.6.2 still requires Click
`~=8.1.0` on Python 3.13, which conflicts with the existing fixed Click floor.
The release README records this unresolved upstream compatibility boundary.
No dependency override, installed-package patch or audit exception was added.

### Bootstrap coverage allowance

The real missing-uv bootstrap was exercised with the official versioned uv
installer, `UV_UNMANAGED_INSTALL`, a checkout path containing spaces, and kcov
43's documented DEBUG method. It installed the pinned Python environment,
completed `rv setup --profile python --no-launcher`, and preserved `uv.lock`.
Kcov measured all nine executable lines of `setup.sh` as covered. The earlier
uv-already-installed run retained three unexecuted installer lines as zero.
Evidence: `artifacts/bootstrap-installer-probe.json` (2026-09-15).

Operator approval (2026-09-21): “Permit the single uncovered line and propose
solutions for the other five items you listed.” This supersedes the proposed
100% threshold above: `verify_bootstrap_coverage` permits at most one uncovered
executable line in root `setup.sh`, including zero uncovered lines. Positive
execution and nonempty, valid, complete native records remain mandatory. The
allowance is scoped to this root bootstrap. The operator supplied no expiry
date; expansion to other scripts or responsibilities requires another decision.

The implementation retains raw covered/uncovered records. Regressions accept
zero or one gap and reject two gaps, empty evidence, wholly unexecuted evidence,
and malformed records. No Sonar source scope, scanner property, server criterion,
other language threshold, or execution count is changed.

Validation: 72 coverage/input/task regressions passed; `rv script-coverage`
passed all four real measured bootstrap cases and published the generic report.
Ruff, strict mypy (247 files), secret scans of changed files and `git diff --check`
passed. This is scoped bootstrap acceptance, not a full CI or Sonar analysis pass.

Keep migration changes in this worktree until required gates pass. Revert tooling
and workflows together. No external release publication or tag changes are part
of migration verification. Existing gate implementations remain until replacement
parity is verified. Native compatibility, active media drift, release dependency
compatibility, and preservation of scanner evidence remain explicit integration
risks; do not resolve them by weakening gates or moving unrelated application
changes into this tooling foundation.

## Dependency rationale

### Proposed completion sequence (2026-09-21)

1. **Repair current blockers.** Update only the unavailable Alpine CA package
   pin after verifying official packages for both target architectures; regenerate
   affected compliance hashes and rerun the real image fixture. For releases,
   upstream PSR commit `fa87c95ffbdaae3b7516dcc355e57bd5f4632c1f` permits
   `click>=8.1.0,<8.6.0` on Python 3.10+, unlike published 10.6.2. Evaluate that
   exact upstream revision in an isolated uv project with fixed GitPython and
   Click, the complete release behavior tests, and auditable dependency exports.
   Prefer a corrected official release for the final pin. A Git-source bridge
   is a proposal, not an installed dependency or a verified compatibility result.
   Do not use a local fork, resolver override or advisory waiver.
2. **Close recipe gaps.** Reconcile each pending row against actual code before
   adding implementation. Port missing database final-proof, shutdown, stack
   contract, advisory and chart checks as static tasks and typed adapters.
   Validate inputs, outputs, failure propagation and cleanup against each
   existing command. Keep internal composition private where no CLI is useful.
3. **Integrate the active media work.** Refresh its exact revision and changed
   tooling requirements. Combine a recorded snapshot with this tooling in a
   disposable worktree, preserving active edits and original commit history.
   Exercise its database proofs, media fixtures/conversion, UI scenarios and
   compliance gates there. Resolve tooling incompatibilities on the foundation;
   recheck any media changes that arrive before handoff.
4. **Complete the cutover.** Convert remaining workflow operations to locked
   `rv` invocations and preserve inputs, secrets handling, artifacts, failure
   gates and ordering. Remove superseded Just/shell/Node entry points once their
   consumers are migrated. Retain root setup.sh and required third-party browser
   assets. Reconcile policies and structured READMEs with the final interface.
5. **Establish final acceptance.** Run full CI and application E2E on the exact
   foundation and combined revisions, including required browser configurations,
   tooling audits/coverage, native coverage, image/compliance and Sonar evidence.
   Use local registries and nonpublishing release validation; real publication
   remains separate. Record commands, source identities and retained reports.
   A green fixture suite alone does not complete the migration.

Database statement boundaries use `pglast` 6.16, the maintained Python binding
to PostgreSQL 16's scanner. This replaces the handwritten Ruby quote/comment
lexer without rewriting SQL or changing the frozen corpus. Its documented
Unicode token positions are converted to UTF-8 byte offsets; the existing
line-aligned prefix rule remains unchanged. A read-only comparison matched all
5,267 boundaries across 167 migrations, the candidate and final initializer
(`artifacts/postgres-scanner-parity.json`, including source hashes). The pinned
PostgreSQL 16.14 server still owns execution and catalog proofs; a scanner match
does not establish semantic parity. The dependency belongs to the `dev` group
and loads only for database tooling. References: [scanner API](https://pglast.readthedocs.io/en/v6/parser.html)
and [PostgreSQL version mapping](https://github.com/lelit/pglast). uv also resolves
its declared `setuptools` dependency (84.0.0); no compiler/development extras are
installed. Core-only CLI startup was verified without pglast installed.

Development uses `watchfiles` 1.2.0 for native filesystem events and batching in
place of cargo-watch, and `psutil` 7.2.2 for process identity, inspection and
bounded termination in place of platform-specific shell process parsing.
`types-psutil` supplies strict static types; watchfiles carries its supported
AnyIO dependency. They are pinned in the existing `dev` group with uv's documented
`add --group dev --bounds exact` interface. Core container commands do not import
these SDKs. The development README describes ownership, settings and failure
behavior, including receipt-based recovery instead of broad port-based killing.

The first native Dockerfile fixture failed on the inherited OpenSSL 3.5.7 package
pins: Alpine's current repository resolution selected development files at 3.5.8.
Keep exact pins and update only `openssl-dev` and `openssl` to `3.5.8-r0`, retaining
their matching runtime declaration and Apache-2.0 license. Both architecture APKs
were downloaded over verified HTTPS from Alpine's official v3.23 repository and
their actual SHA-256 values were recorded in the declaration. Evidence:
`artifacts/container-openssl-pin-failure.log` and
`artifacts/openssl-3.5.8-packages.json`. Upstream package record:
[Alpine OpenSSL 3.5.8](https://pkgs.alpinelinux.org/package/v3.23/main/aarch64/openssl).
No package trust bypass, unversioned fallback or runtime-package removal is used.

The runtime stage then exposed the inherited curl 8.20.0 pin's removal from the
current index. A complete check of all 28 builder/runtime pins against both
architectures' official main/community indexes identified curl as the sole
remaining unavailable version. Update it to exact `8.22.0-r0` and replace its
two actual APK hashes in the runtime declaration. Its use remains the existing
HTTP health request. Evidence: `artifacts/alpine-pin-availability.json`,
`artifacts/container-curl-pin-failure.log`, and `artifacts/curl-8.22.0-packages.json`.
Upstream records: [Alpine curl](https://pkgs.alpinelinux.org/package/v3.23/main/aarch64/curl)
and [curl 8.22.0 changes](https://curl.se/changes.html#8_22_0).

Tree-sitter's Python bindings (`tree-sitter` 0.26.0) and official JavaScript
grammar (`tree-sitter-javascript` 0.25.0) supply syntax locations for classic
scripts and ES modules. Chromium's standalone compile command accepts classic
scripts only; source inventory must also cover Revaer's vendored modules without
executing them or resolving their network imports. The maintained native grammar
replaces that limited inventory mechanism. V8 still supplies every execution
counter; parsing alone never marks a line covered. Both packages are installed
and locked through uv's project commands and have upstream platform wheels.

uv and uv_build provide locked interpreter/environment provisioning and packaging. pytest and pytest-playwright provide E2E fixtures; pytest-xdist, pytest-rerunfailures and pytest-html preserve workers, retries and HTML reports. pytest-timeout retains the browser suite's per-test deadline using the maintained plugin's signal mode on supported Unix hosts. Its 2.4.0 pin avoids the withdrawn 2.5.0 release. pytest-cov supplies real Python coverage. Python Semantic Release replaces the Node release engine. PyYAML reads workflow and chart configuration; jsonschema validates API data against OpenAPI components. Ruff, mypy and pip-audit enforce format/lint, types and advisories; stub packages provide static checking for YAML/JSON schema dependencies. Standard-library code handles dispatch, processes, files, hashing and configuration.

## Stale-policy check

On 2026-09-21 the operator explicitly excluded `.env` files from pre-read secret
scans and confirmed the reported passwords are local testing values. Root
`AGENTS.md` now records that exception and requires explicit file enumeration
instead of recursive scans that would include `.env`. The reported Python
default was inspected: it supplies a Helm rendering URL and does not establish
a database connection. The two findings in the workflow-contract fixture are
local E2E database URLs. The testing README now explains these boundaries.
Source scanning remains enabled. No scanner property,
quality gate, or remote finding disposition was changed, and no blanket
exception was added for unfamiliar credentials.

Reviewed root AGENTS.md and devops, Rust, UI and data instructions. Main still mandates Just and contains older Sonar scope relaxations; the user explicitly authorized replacing the command surface, not weakening quality. Final policy changes must retain the stricter user-supplied contract and current media-stack enforcement. Sonar scoped instructions must be reviewed when migrating scanner inputs.

### Additional implementation verification

On 2026-09-21, Python-only root bootstrap passed. A fresh `rv tooling-check`
passed formatting, Ruff and strict mypy, then reported 1418 passing tests and
four failures. Three failures were stale assertions for the capitalization of
the browser-selection diagnostic; the corrected assertions retain the invalid
input checks, and all 16 setup-selection tests passed on rerun. Ruff and format
checks passed for the changed test file.

The remaining container failure is an unavailable exact Alpine runtime pin:
`ca-certificates=20260611-r0`; the repository now offers `20260909-r0`.
No package constraint was relaxed. The complete suite is not green, and the
separate release dependency audit blocker above remains unresolved. These fresh
results supersede the earlier complete-suite pass as current acceptance evidence.
The pre-read scan omitting `.env` reported only the confirmed Helm and E2E
testing URLs. The changed instruction, README and ADR files passed secret scans,
and `git diff --check` passed. Full project CI/E2E was not rerun in this pass.

Full foundation `rv lint` passed in the supported Linux environment, including
both Clippy modes, at `artifacts/foundation-attempts/baf214d45ff41a9b`. Native
compilation database generation passed at `8581bfb556162dea`, with tooling SHA-256
`1f5c16027f02ed485cadb678a3f86410dce54b7b14e5f462822b89f5d27db6b1`.

Real Buildx tests cover local image loading, two-platform OCI output, paths with
spaces/commas, failed-build artifact invalidation, and preserving the selected
builder. Real Trivy scanning detects a disposable private key and retains its
failing JSON report. A local TLS/authenticated Distribution registry verifies
byte-identical Helm chart/provenance and Artifact Hub metadata publication;
incorrect credentials or absent certificate trust fail. Helm and ORAS use their
official scoped registry configuration and CA options, with no trust bypass.

The old foundation exposed six vulnerabilities and seven denied warnings. The
tooling branch now follows the media stack's existing prerequisite history through
`ec8adc14`, which already supplies the dependency and native ABI fixes. Reusing
those commits avoids competing fixes and leaves the active media work untouched.
Four additional lockfile versions match updates already present in the media
integration: h2 0.4.16, rustls 0.23.45, rustls-webpki 0.103.15, and chacha20 0.10.2.
The first three address current security advisories; the last replaces a yanked
release. No manifest constraint or advisory ignore was added.

`rv audit` and `rv deny` passed on this shared baseline on 2026-09-15. Logs are
`artifacts/shared-baseline-audit.log` and `artifacts/shared-baseline-deny.log`.
Earlier application and coverage proofs above remain evidence for their recorded
source revisions; full CI and E2E acceptance on the new baseline and combined
media checkout remain outstanding.

### Dependency blocker progress (2026-09-21)

Updated the unavailable CA package to exact `20260909-r0`. Downloaded both
Alpine v3.23 architecture packages over verified HTTPS and recorded actual
SHA-256 values in the runtime SPDX declaration. Evidence is retained in
`artifacts/ca-certificates-pin-verification.json`; the real Dockerfile fixture
and build-input/compliance tests passed in `artifacts/ca-pin-validation.log`.

The approved upstream PSR revision resolved in a disposable uv project with
GitPython 3.1.62 and Click 8.5.0. Release preview/publication-fixture/workflow
tests passed (`artifacts/release-upstream-validation.log`). No real publication
occurred. The Git-source requirements export fails the unchanged hash-required
audit. The supported source-archive form supplies a hash, but pip-audit reports
that it skips URL requirements, despite returning success. This is incomplete
audit evidence; the candidate has not been promoted to the project lockfile.
The next step is to retain uv's exact source/hash verification and establish
complete advisory coverage for the installed candidate and all locked groups
through supported auditing interfaces, with regressions that reject skipped
packages. No exclusion or advisory ignore is authorized by this experiment.

### Release dependency blocker resolved (2026-09-21)

uv's supported PEP 751 export and pip-audit's native strict lockfile input
retain both source identity/hash and advisory package versions. The upstream
archive candidate audited all 89 third-party packages without skips or findings.
Promoted that exact archive with uv, removed the vulnerable GitPython upper
constraint, retained fixed-version floors, and reran `rv tooling-audit` successfully.
The gate now rejects skipped, malformed, empty and vulnerable successful-process
responses; it retains audit JSON and invalidates prior success before execution.
No additional Python dependency or custom lockfile conversion was introduced.
The source archive remains temporary until an official release includes both
upstream compatibility fixes. Earlier blocker entries above are historical.

Promoted-lock verification: 60 release tests, 48 adapter/setup tests, and two
real dependency-group/container tests passed. Ruff, formatting, strict mypy and
secret scans passed. `rv tooling-audit` passed with retained PEP 751 and JSON
evidence under `artifacts/python-audit/`; the release and dependency-group logs
are `artifacts/release-promoted-validation.log` and
`artifacts/dependency-groups-pylock-validation.log`. These checks resolve the
dependency blockers; remaining recipe cutover and full integration acceptance
are still required.

### Runtime shutdown recipe migration (2026-09-21)

Compared the active operator-workflow quality recipes and added both missing
commands to the static rv registry. Typed Cargo methods preserve library/test
selection, shown output, all-feature and no-default-feature runs, and the four
existing Clippy passes. Positive execution is required in each test mode, so
removed tests cannot silently pass. No new dependency or lint suppression was
added. Native fixtures cover both modes, missing selections, test failures and
minimal-mode compiler warnings. The fixture needed full workspace package
metadata for Clippy; its first failure was a fixture deficiency, not permission
to relax the gate. The task README and Rust scoped instructions record these
contracts. Actual media application acceptance remains pending integration.

Runtime task verification passed 42 native/adapter/dispatch tests, Ruff, strict
mypy (247 files), secret scans and whitespace checks. Evidence is retained in
`artifacts/runtime-shutdown-tooling-validation.log`. The strict type pass also
found and corrected a local variable-name collision in the pylock export test.

### Stack and advisory contract parity (2026-09-21)

Comparison with the active operator-workflow recipes found missing media checks
in the Python required-check validator. Added canonical conversion, required
report upload under always(), cleanup under always() after upload, and both
image/release dependencies. Release jobs cannot skip pull requests. Independent
mutations cover each boundary; existing supply-chain validation remains shared.
The three original advisory failure cases now exercise policy, audit and deny
entry points. These tests belong to tooling-check, replacing standalone test
recipes without adding unnecessary CLI commands. All 87 focused tests passed.
No dependency or gate relaxation was introduced. Updated policy README, devops
instructions and recipe mapping; application/workflow acceptance remains pending.

Inspection also confirmed the media compliance-chart suite includes digest,
architecture, PVC/subPath binding and adversarial values absent from foundation
packaging tests. Keep that recipe pending until those checks are ported and run
against the integrated chart. Final database proof composes multiple native
qualification modules; its port must preserve the full composition, not only
the top-level Ruby entry point.

### Native chart compliance port (2026-09-21)

Ported the complete Ruby renderer suite: ten accepted and ninety rejected cases,
including separate schema-only and template-only validation, duplicate YAML keys,
immutable digest/architecture/PVC/subPath binding and retained unrelated settings.
Added two parser regressions. The source chart fixture is an unchanged snapshot
from f4b80bf76043c03091a1860c5c4b3de33ed256fe and remains in scanner scope.
The static compliance-chart-test task always targets the selected checkout;
HelmLint composes it for charts that contain the media compliance contract.
There is no fixture fallback in the public task and no Kubernetes publication.
The first port run exposed overly-specific diagnostic matching for reserved
annotations/selectors; restoring the original field-level matching preserves
the native validation assertions. No dependency or chart behavior was changed.
Updated fixture/task READMEs, devops instructions and the recipe inventory.

Chart verification: 102 cases passed against the recorded fixture and 102
against the active operator-workflow chart, without modifying it. The existing
18 packaging/dispatch tests also passed. Ruff, strict mypy (248 files), secret
scans and whitespace checks passed. Evidence: `artifacts/chart-compliance-python-validation.log`,
`artifacts/chart-compliance-media-validation.log`, and
`artifacts/chart-port-regression.log`. These are renderer and tooling proofs;
full combined application/image acceptance remains outstanding.

### Final database proof: extension stage (2026-09-21)

Ported the complete extension boundary from the current media proof into an
injected Python component. All 27 original checks retain stock catalog identity,
40 routines/two callbacks, eleven mutations with exact rollback, and runtime
primitive calls. Native tests also force a query failure before rollback and
verify a changed target fails comparison. Existing typed PostgreSQL operations
and owned resource cleanup are reused; no dependency or application SQL changed.
Private stock/observed evidence is retained without claiming whole-proof success.
The final-proof CLI composition and other native/ingestion stages remain pending.
Reviewed database instructions, package README and migration inventory together;
added the extension-specific invariant and kept incomplete status explicit.
Rollback is removal of this port before workflow cutover; existing Ruby remains
available until the complete replacement has been validated.

Validation: 52 focused database extension/lifecycle/candidate tests passed,
including real PostgreSQL execution and cleanup. Ruff and strict mypy (250
source files) passed. Secrets scans were clean. Evidence is retained in
`artifacts/database-extension-validation.log`. This does not replace the
remaining whole-application, final-proof or Sonar acceptance gates.

### Final database proof: reset timeout stage (2026-09-21)

Ported all seven reset-timeout checks using the injected PostgreSQL adapter and
executor/timing/identity collaborators. Real workers prove cancellation and lock
contention; cancellation rechecks the unique application, database, sleep event
and PID. The bounded lock holder is explicitly canceled and rolled back instead
of maintaining a shell-driven interactive psql stream. The five-second reset
bound and outer settings are unchanged. Workers are joined before observer
cleanup; private transcripts retain failure diagnostics. No dependency was added.

The initial native runs passed against both a minimal reset fixture and the
actual operator-workflow initializer, applied/sealed only in disposable storage.
The production initializer and active media worktree were not modified. Added
negative checks for a removed routine timeout and warning cleanup. Package README,
database instructions and migration inventory were reviewed and updated together.
Remaining native/ingestion proof stages and final command composition are still
pending. Rollback before cutover is removal of this port; the prior command stays
available until complete replacement is proven.

Final validation: 30 focused native reset/extension/lifecycle tests passed;
the updated reset test also passed against the actual media initializer,
including timeout-removal rejection and warning cleanup. Strict mypy (252 files),
Ruff, formatting, secrets and whitespace checks passed. Logs:
`artifacts/database-reset-validation.log` and
`artifacts/database-reset-media-validation.log`. GitHub source refresh timed out
during verification; remaining local checks used uv offline with the unchanged
locked cache. This is not a new dependency-audit or full integration result.

### Final database proof: baseline stages (2026-09-21)

Ported 53 baseline checks: invalid/unsealed/repeated seals, exact shape/digest,
malformed reads and rollback, runtime/outsider/surrogate permission rejection,
application procedure call paths, post-seal atomicity, timeout preservation and
runtime reads after bootstrap login removal. Native queries use the existing
injected connection and typed adapter. Denial matching now requires the actual
error field and exact DETAIL; misleading text cannot satisfy a rejection. The
schema-GRANT warning retains its original diagnostic-plus-unchanged-ACL assertion.
No dependency or application initializer change was introduced.

All 53 checks passed on the active media initializer. A subsequent composed run
passed 95 checks across baseline, extension and reset stages, including repeated
runtime calls after bootstrap login removal. Both native resource receipts prove
completion and storage removal. The exact tested initializer SHA-256 is
`ae8b62e47bc7745d6c5db14387cfd6a09dec8a5fcd56b72f264ee480049c6888`;
this is current working-tree stage evidence, not frozen-contract verification or
permission to revise a pin. Thirteen diagnostic/identity regression cases also
passed, along with Ruff, strict mypy (254 files), secrets and whitespace checks.
Evidence is in `artifacts/database-baseline-proof`,
`artifacts/database-composed-stages`, and their corresponding validation logs;
`artifacts/database-baseline-diagnostics.log` records focused regression results.

Reviewed and updated the database instructions, package README and migration
inventory. Schema/seed parity, routine privilege inventory, native ingestion and
complete command composition remain open. The existing full Ruby proof remains
until its whole replacement is validated. Rollback is removal of these stages
before workflow cutover; private proof evidence remains useful for diagnosis.

### Final database proof: routine permission inventory (2026-09-21)

Ported the remaining six permission assertions from the final proof into
BaselineProof: owner/definer/trigger hardening, exact grant count, exact identities
and ordered settings, PUBLIC execution, relation privileges and database
privilege matrix. The stage receives candidate-classified Routine records and
retains all native rows, including extensions, in a private JSON artifact.
Explicit identity-set equality also rejects substitutions with the same count.
Missing extension classification fails rather than silently hiding a routine.

A pinned native PostgreSQL fixture passes the six checks, independently mutates
eleven privilege/identity/settings properties, then verifies exact restoration
following each mutation. The initial test found that resetting and re-adding a
function setting changes catalog order; the fixture now restores the original
order without weakening the assertion. Fourteen focused tests passed, including
the existing denial-diagnostic regressions. Strict mypy (255 files), Ruff, secrets
and whitespace checks passed. Evidence:
`artifacts/database-routine-permissions-validation.log`.

Updated the package README, database instructions and migration inventory. No
new dependency, application SQL change or scanner/gate exception was introduced.
This is native fixture evidence; validation against the actual frozen candidate
and media initializer remains part of the complete proof integration. Rollback
before cutover is removal of this addition while retaining the original proof.

### Final database proof: schema and seed parity (2026-09-21)

Ported the exact dump/header and seed-identity normalizations, reusing approved
legacy routine deltas and typed dump/query operations. The catalog inventory
uses JSON instead of delimiter-split metadata. Both compared schema and seed
inputs remain private artifacts; previous pairs are removed before execution,
so malformed seed input cannot leave stale evidence. No dependency was added.

Native tests replay the reference as postgres and final SQL as a constrained
owner, then prove detection of table/seed/UUID drift. The reference fixture had
to preserve the real ingestion body's nested dollar quotes to reproduce native
pg_dump delimiters; the approved transform was not broadened. Nine focused tests
passed, including preservation of function-body text and unrelated settings.
Full media parity still requires the reviewed frozen corpus and is not claimed
from these fixtures. Updated package README, database instructions and migration
inventory; no gate or hash pin changed. Rollback before cutover is removal of this
stage, retaining the existing full proof until its replacement is complete.

Database regression milestone: all 174 database-tooling tests passed in the final
run (`artifacts/database-tooling-regression.log`), including native parity and
owned resource cleanup. Ruff, strict mypy (257 files), formatting, secrets and
whitespace checks passed. The initial broad run had one managed-startup timeout
and 173 passes; its isolated recheck passed, followed by the clean full rerun.
No timeout or ownership gate was relaxed. The cause of that transient timeout
was not established. The original failure remains in
`artifacts/database-tooling-regression-initial.log`; its generated disposable
fixture password was redacted, and the retained log then passed secrets scanning.
Focused parity evidence is in `artifacts/database-parity-validation.log`.

### Ingestion proof: shared evidence boundary (2026-09-21)

Ported strict session/diagnostic framing and committed-identity/clock
normalization into the ingestion package. The parser receives the independently
verified frozen body/signature and runtime role. It rejects role substitution,
privilege escalation, missing/malformed frames, changed caller settings and
helper-order/answer drift. Native diagnostic signatures, embedded statements and
source lines remain bound to the frozen body; only the verified directive's
one-line difference is normalized. Duplicate-key evidence is rejected.

Sixteen focused tests passed, including actual PostgreSQL success/error sessions
for both reference and final bodies, equal normalized source lines, and forged
diagnostics/identities. Evidence: `artifacts/ingestion-evidence-validation.log`.
No dependency, application SQL or gate criterion changed. Package and parent
READMEs, database instructions and migration inventory were reviewed and updated.
The source proof explicitly reports complete D3 scope as unproven; this port
preserves that distinction and does not convert passing cases into approval.
Producers, application/native qualification cases and final composition remain
pending. Rollback is removal of this stage before replacement of the original.

### Ingestion proof: session producer and controls (2026-09-21)

Ported the 24 named fixture arguments, thirteen cold/helper-first/warm cases,
psql session producer and three native transaction controls. Unknown arguments
and empty helper SQL fail. The owning composition injects helper SQL, connection
and check collector. Controls use the exact producer and prove successful second
commits, changed second writes and rollback of only the failed second write.
Private SQL/stdout/stderr are retained; old diagnostics are invalidated before
native execution. Exact state/diagnostic matching remains mandatory.

Twenty-four focused session/evidence tests passed, including real PostgreSQL
transactions and cleanup. Evidence: `artifacts/ingestion-sessions-validation.log`.
Reviewed and updated ingestion/parent READMEs, database instructions and migration
inventory. No dependencies, application SQL or gate criteria changed. These
controls establish harness behavior; isolated case producers, application/native
qualifications and the explicitly unproven complete D3 scope remain outstanding.
Rollback is removal of this port before replacing the original full proof.

### Ingestion proof: verified native inventory (2026-09-21)

Ported inventory collection for the twelve reviewed routines and triggers on
eighteen ingestion tables. Both distinct databases must report PostgreSQL 16.14
and the exact helper set. Bodies/signatures match except for the existing approved
ingestion deltas and local directive, with the required reference setting and
absence of that setting on the final routine. Existing byte transformations are
reused. VerifiedInventory binds the evidence parser to the checked reference
body/signature and retains both private snapshots; no positive check is emitted
before verification completes. No dependency or application SQL changed.

Twenty-five combined ingestion tests passed. Native catalog tests cover accepted
transformations, eight independent snapshot mutations and live body drift that
prevents a positive result, with owned storage cleanup. Evidence:
`artifacts/ingestion-inventory-validation.log`. Updated ingestion/parent READMEs,
database instructions and migration inventory. This validates the inventory
component, not the complete application/native qualification or outstanding D3
scope. Rollback remains removal of this port before full workflow cutover.

### Ingestion proof: isolated execution (2026-09-21)

Ported per-case reference/final clones, injected seed application, eighteen-table
snapshots, native query evidence, strict parsing and comparison normalization.
Case execution uses typed query transport and explicitly selected principals.
Only successfully created clones are removed; an existing-name collision is not
adopted. Success returns after clone cleanup. Failure diagnostics retain cleanup
and earlier errors, while the outer proof still owns all container storage.
Prior SQL/output/table artifacts are invalidated and replacements remain private.

Twenty-six combined ingestion tests passed. Native isolation tests prove matching
normalized reference/final observations, unchanged source data, cleanup after
seed/SQL/warning/parser failures, collision preservation and safe artifact names.
Evidence: `artifacts/ingestion-isolation-validation.log`. No dependency, application
SQL or gate criterion changed. Updated ingestion/parent READMEs, database
instructions and migration inventory. Actual application cases, approved-outcome
comparisons, native instrumentation and full composition remain pending. Rollback
is removal of this port before replacing the original full proof.

### Ingestion proof: exact approved D4/D5 corrections (2026-09-21)

Ported the complete approved rejection shapes, warm refresh and external-ID
expected changes, including the required existing-case semantic assertions.
The gate compares full observations and preserves diagnostic statement hashes,
signatures, lines and native locations. Unknown cases/differences receive no
approval; malformed inventory fails. Structural comparison preserves Ruby's
numeric equality while rejecting Python boolean/integer coercion. No approval,
dependency, application SQL or gate criterion changed.

Recorded independent expected fixtures using the original Ruby transformations
and statements recovered from frozen migration 0052. Both statement hashes match
the approved constants. Fixture provenance records source hashes and explicitly
labels synthetic evidence. Tests reject changed diagnostics, result flags, rows,
clocks, caller settings, boolean types and missing tables, and verify no mutation
of inputs. These are comparison proofs, not full application qualification.
Updated ingestion/parent READMEs, database instructions and migration inventory.
Application/native producers and complete composition remain outstanding. Rollback
is removal of this port before full workflow cutover.

Validation: all 53 ingestion tests passed; Ruff, formatting, strict mypy
(268 files), secrets and whitespace checks passed. Retained evidence:
`artifacts/ingestion-corrections-validation.log`.

### Ingestion proof: thirteen-case composition (2026-09-21)

Connected the cold/helper-first/warm case definitions, session producer, isolated
runner and exact correction matcher. Each pair must be admissible and meet the
expected final SQLSTATEs; identical failures are still failures. The first failed
case stops execution. A private phase receipt replaces stale output and retains
partial results and interruption/transport reasons. Phase completion is separate
from complete D3 qualification, whose complete/passed fields remain false.

All 59 ingestion tests passed, including injected matrix acceptance/failure cases
and the existing native component checks. Strict mypy (270 files), Ruff, formatting,
secrets and whitespace checks passed. Evidence: `artifacts/ingestion-matrix-validation.log`.
The matrix composition tests are not actual application acceptance; full media
execution and other application/native qualifications remain outstanding. Updated
package/parent READMEs, database instructions and migration inventory. No dependency,
application SQL or gate criteria changed. Rollback is removal of this composition
before replacing the original full proof.


### Current media phase and qualification correction (2026-09-21)

Live source review found `feature-development` in the media manifest and Ruby
contract, but missing from the Python phase enum. Ported its actual freeze
behavior: retain the exact historical corpus and input validation, require a
regular initializer with scannable statements, and permit initializer evolution
without rewriting or enforcing the historical finalization hash. Explicit
candidate and finalization checks retain their separate exact-byte contracts.

The accepted media ADR 591 and current data instructions expressly supersede
legacy-equivalence qualification for v0 and prohibit restarting the historical
D3 proof project. Recent ingestion porting therefore exceeded the relevant
qualification scope. Stop expanding that project. Preserve existing work pending
bounded retirement; reuse baseline/extension/reset checks for fresh-init,
least-privilege, recovery and complete real-application qualification. No claim
of completed D3 or v0 release readiness is made. No other gate is relaxed.

Validation: 56 focused contract/scanner tests passed, including feature edits,
corpus drift, malformed/missing init and unchanged pins. Strict mypy passed all
270 files; Ruff and deterministic secrets checks passed. Read-only verification
of the live media tree matched its frozen 167-file corpus and left its manifest
unchanged. Exact initializer hash and scope are retained in
`artifacts/database-feature-phase-media.json`; initializer bytes have changed
since earlier runtime-stage evidence, so that evidence is not current acceptance.
Reviewed database instructions, migration inventory and package README; corrected
stale migration/parity direction there. No dependencies or application SQL changed.
Rollback is reverting phase support and documentation; no media state was mutated.


### PR media workflow executor cutover (2026-09-21)

Migrated only the PR media-conversion job to the existing Python cache-key,
fixture acquisition/generation, native conversion, report and cleanup tasks.
Preserved its required job name, dependencies, report/upload/cleanup conditions,
artifact path and missing-report failure. Removed the retired setup toolchain
input; setup selects the repository toolchain. Cache identity now consumes the
actual `tool_version` and `fixtures` outputs, including Python implementation
bytes. The internal conversion step label matches the existing media contract.

Validation: 133 focused media task, automation and workflow tests passed,
including the actual edited job against existing prerequisite/order/artifact
contracts. Strict mypy passed 270 files; Ruff, formatting, secrets and whitespace
checks passed. Evidence: `artifacts/pr-media-cutover-validation.log` and
`artifacts/pr-media-cutover-types.log`. Hosted application execution remains
pending; injected/native fixture checks are not a completed CI run.

A broader PR/Sonar rewrite was rejected by automatic approval review because
it combined gate/condition changes. It did not execute. The bounded media-only
alternative was approved and executed; PR scanner and Sonar workflows remain
unchanged pending separately reviewed, narrowly scoped edits. No dependency or
gate criteria changed. Reviewed/updated devops instructions and task record.
Rollback restores this job's previous executors; no external workflow was run.


### Strict chart lint and pending PR caller patch (2026-09-21)

Restored `--strict` in the typed Helm lint adapter to match the media packaging
recipe. A native deprecated-API fixture is structurally valid and succeeds in
ordinary lint; the packaging task rejects its warning and creates no output.
All 12 native chart tests pass. No dependency, signing or publication behavior
was added. Updated the tooling README and devops instruction; rollback is the
adapter/test change, though dropping strictness would reintroduce the gap.

The nine ordinary PR job executor changes remain unapplied after automatic
approval review rejected the grouped security/release-related workflow edit.
Prepared `/private/tmp/revaer-pr-ordinary-jobs.patch` for explicit approval.
Structural validation proves job metadata, conditions, dependencies, unrelated
steps and scanner jobs unchanged, and validates the proposed CLI/setup inputs.
The patch has not run any workflow. A full tooling regression run is in progress
with evidence at `artifacts/tooling-regression-current.log`; it is not yet green.


### Media chart packaging integration and full tooling result (2026-09-21)

Ported the media recipe's synthetic lint image/architecture/compliance inputs
into a private values file outside the chart. They apply only to charts exposing
the compliance contract; foundation behavior is preserved. Actual packaged
values remain byte-identical to source, and no lint-only file enters the archive.
Thirteen native chart tests pass, including strict-warning and media packaging
cases. A disposable copy of the current active media chart also packaged and
verified with unchanged source/default hashes. No chart was published. Evidence:
`artifacts/helm-media-package-validation.log`, `artifacts/helm-current-media-validation.log`
and `artifacts/helm-current-media-receipt.json`. Strict types, Ruff and formatting
passed. Updated README and devops rules; no dependency or gate relaxation.

The completed full tooling run reported 1,643 passed and one failed managed
PostgreSQL readiness test in 385.61 seconds. Its isolated rerun passed in 4.21
seconds. This does not establish a cause or clear the full-suite failure.
Retain `artifacts/tooling-regression-current.log` and
`artifacts/managed-database-recheck.log`; investigate startup diagnostics before
claiming the milestone green. No test was disabled or timeout extended.
The nine-job workflow patch remains unapplied pending the requested approval.


### Managed PostgreSQL failure evidence (2026-09-21)

Added bounded fixture diagnostics before owned-container cleanup: Docker state
only (excluding credential-bearing configuration) and the last 100 log lines,
with generated password/raw-URI encodings redacted. Cleanup remains in finally,
so a diagnostic failure cannot leave the fixture container behind. Production
readiness timeouts and ownership checks are unchanged.

The complete database test file reproduced the startup failure. PostgreSQL
completed initdb and stopped its temporary server, then the final server exited
with `data directory has wrong ownership`. This is stronger evidence than the
previous TCP timeout. The retained directory reported host ownership 501:20 and
a subsequent read-only Docker mount also reported 501:20, matching the failed
container's configured user. The underlying transient mismatch is not yet
explained; do not call it fixed or weaken the ownership gate. The fixture removed
the failed container. Evidence: `artifacts/managed-database-startup-exit.log`
and `artifacts/managed-database-diagnostics-validation.log`. Formatting and type
checks passed; the lifecycle suite remains failed until the cause is addressed.


### Ownership comparison: no demonstrated fix (2026-09-21)

Compared six fresh starts with container-created PGDATA and six with host-created
PGDATA using the existing Docker adapter, numeric UID/GID, bind storage, read-only
root filesystem and the same PostgreSQL image. All twelve reached TCP readiness;
none reported wrong ownership. Every disposable container and data directory was
removed. This sample does not justify precreation, retries, a timeout increase or
a storage change as a fix. Evidence: `artifacts/postgres-ownership-comparison.json`
and `.log`. Image ID was
`sha256:7e7dbab8d3b431a20793a6d99cb5a6bc84e44914309917f1bf5589a7568cdefd`,
repository digest `sha256:57c72fd2a128e416c7fcc499958864df5301e940bca0a56f58fddf30ffc07777`,
Docker server 29.8.0. No production code or Docker settings changed.

Upstream Docker issue https://github.com/docker/for-mac/issues/7415 documents the
same symptom despite matching stat ownership on an older Desktop release. It is
corroborating context, not proof of this host's cause or authorization to change
its global filesystem backend. The full-suite failure remains unresolved.


### Exact Helm annotation marker parity (2026-09-21)

The original AWK renderer requires exactly one standalone marker comment. The
Python package path previously selected the first substring match and could
accept duplicate or embedded markers. Extracted a pure renderer preserving the
unique-comment contract and authored neighboring YAML, with multiline metadata
retained as YAML strings. Invalid templates fail before output replacement.

Validation: 22 annotation/native package tests passed, including original
multiline semantics, whitespace variants, missing/duplicate/embedded markers,
unchanged source and existing-package preservation. Strict types passed all 271
files; Ruff, formatting, secrets and whitespace checks passed. Evidence:
`artifacts/helm-annotation-validation.log` and `artifacts/helm-annotation-types.log`.
No dependency or workflow changed. Updated chart README, devops rules and recipe
inventory; broader prerequisite composition remains pending. Rollback is the
renderer and its callers/tests, with the original malformed-marker gap restored.


### Helm lint prerequisite composition (2026-09-21)

Added static HelmAnnotationTest and HelmPackageTest tasks and CLI dictionary
entries. HelmLint preserves annotation, applicable media compliance and package
prerequisites before unsigned packaging. Package tests now exercise HelmPackage
directly with explicit unsigned settings, avoiding recursion through HelmLint.
Composition tests prove execution order and that failures stop before packaging.

Sixty focused chart tests passed. The actual `uv run --offline --locked -- rv
helm-lint` command also passed its prerequisites and packaged the foundation
chart locally; no artifacts were published. Strict mypy passed 272 files, with
Ruff, secrets and whitespace checks clean. Evidence:
`artifacts/helm-prerequisites-validation.log`, `artifacts/helm-prerequisites-types.log`
and `artifacts/helm-lint-cli-validation.log`. The current media chart's full
composed command remains to be exercised, although its native packaging has
already passed. Updated README, devops rules and inventory. No new dependencies
or gate relaxation. Rollback is task registration/composition and test separation.


### Current-media chart composition and docs recipe restoration (2026-09-21)

The composed HelmLint task passed against a disposable copy of the current media
chart: six annotation, 102 selected-chart compliance and 47 package/version
checks, then strict unsigned packaging and metadata verification. Source/default
hashes remained unchanged. Injected external adapters used the tooling checkout
for regression sources; this is composed-task evidence, not a fully rebased media
CLI/application result. Temporary output was removed; no publication occurred.
Evidence: `artifacts/helm-current-media-composition.log` and `.json`.

Comparison with media's six docs recipes found missing docs-install and automatic
Lychee installation. Added DocsInstall static CLI dispatch using shared pinned
Cargo selection and a typed MdbookMermaid adapter for its native install command.
Docs now composes install/build/index; link checking installs its pin first.
Build and serve retain pin verification; index retains release mode. No new
packages or dependency versions were introduced.

Native fixtures verify Mermaid book integration, generated book and index output,
and broken/resolved local links. Both docs-focused checks passed; strict mypy,
Ruff, formatting, secrets and whitespace checks passed. Evidence:
`artifacts/docs-media-comparison.log` and `artifacts/docs-media-types.log`.
Current-media book/index acceptance remains pending. Updated README, devops
instructions and all six inventory rows. Rollback removes the task/adapter and
restores earlier composition, reintroducing the installation gap.


### Build/test/image recipe comparison and license prerequisite (2026-09-21)

Compared current media quality, release and image recipes with the Python tasks.
Rust test variants preserve workspace/all-feature selection, the single-threaded
native libtorrent suite, API/app no-default-feature tests and database overrides.
Validation retains ordered gates followed by release build, with additional
Python coverage. Native composition tests preserve ownership locks and failure
propagation. Build/release/SBOM/export operations retain required selectors;
release-lock already maps to the unified uv lock. Image tasks retain single-load
versus multi-platform export and the HIGH/CRITICAL scan gate. Reused existing
native Buildx/Trivy evidence rather than rerunning unchanged expensive scans.

Found and restored the license recipe's pinned cargo-deny installation before
native JSON reporting. Thirty-three build, Rust-selection and composition tests
passed, including native license output. Strict mypy passed 272 files; Ruff,
formatting, secrets and whitespace checks passed. Evidence:
`artifacts/build-test-media-comparison.log` and `artifacts/build-test-media-types.log`.
Updated fourteen inventory rows with precise fixture-versus-application scope;
full media application builds, suites and image qualification remain pending.
README and devops rules document standalone installation. No new dependencies,
workflow changes or gate relaxation. Rollback removes the installer prerequisite,
restoring the standalone-reporting gap.


### Disposable current-media integration snapshot (2026-09-21)

Created detached worktree `/private/tmp/revaer-rv-media-integration` at media HEAD
`f4b80bf76043c03091a1860c5c4b3de33ed256fe`. Copied all 128 current media changes,
verified every copied file and rechecked source hashes, then recorded snapshot
tree `abddbfb2a6dba8988e1096619acf5410d9336dc6`. Applied the existing tooling
tracked diff with Git's three-way merge and copied 242 new tooling files with
collision/hash checks. Neither active worktree was modified.

There are 18 unresolved paths: setup action, build inputs, four instruction files,
five workflows, Cargo.lock, two docs indexes, OpenAPI/LLM generated outputs and
tests README. Integration is not runnable acceptance while conflicts remain.
Do not resolve policy/workflow conflicts by overwriting the media side. The
separately rejected nine-job workflow proposal was not applied; approval is still
pending. Preserve this disposable checkout for deliberate conflict resolution.
Evidence: `artifacts/media-integration-snapshot.json`,
`artifacts/media-integration-conflicts.json`, `artifacts/media-integration-apply.log`
and source-scan logs. Media changed files scanned clean. Tooling scan findings
were the already-approved local test credentials; their values were not printed.
No dependencies were resolved, tests run or external artifacts published here.
Rollback removes only this disposable integration worktree and its receipts.


### Six integration conflicts resolved; Sonar wording corrected (2026-09-21)

In the disposable integration checkout, preserved both ADR histories in the two
indexes, retained the migrated Python E2E guide, and resolved the five lockfile
edge conflicts to the media selections. `cargo metadata --locked --offline`
accepted the complete resulting graph without changing dependencies. Build inputs
retain all media PostgreSQL pins plus reviewed uv/Python and updated Alpine pins;
retired Node/npm/Just inputs are removed as already specified by the migration.
The Rust instruction conflict contained equivalent root-precedence statements;
retained the media wording. Six paths are staged as resolved, twelve remain.
Evidence/hashes: `artifacts/media-integration-resolution.json` and the retained
Cargo metadata under `/private/tmp/revaer-integration-cargo-metadata.*`.

The conflict review exposed stale decoration-only Sonar wording in the tooling
branch's devops instructions. Replaced it with the existing media requirement:
scanner waiting, retained report/task evidence and same-analysis API verification
remain mandatory. This strengthens stale documentation; no workflow, server
setting or gate criterion was changed. The blocked nine-job proposal remains
unapplied. Integration is still unqualified while other conflicts remain.

### Integration manifest validation and database policy merge (2026-09-21)

Removed two identical PostgreSQL assignments introduced during conflict resolution;
all 23 remaining build settings pass the production literal-manifest parser.
The seven-path resolution receipt now exists and retains exact source hashes.
Database policy preserves both sets of historical proof safeguards under their
explicit historical-only headings and keeps ADR 591 single-init authority.
Updated its command references to `rv`; no frozen SQL or runtime code changed.
The nine-job workflow proposal remains unapplied. Full integration is pending.

The Sonar policy conflict is also resolved in the disposable checkout: retain
media scanner waiting, same-analysis verification, zero issues/hotspots and all
strict source rules; add the approved bootstrap allowance and Python property
parser checks. Removed conflicting decoration-only/backlog-exemption guidance.
Eight paths are now resolved and ten conflicts remain. No workflow or remote
criteria changed. The resolution receipt includes the final policy hash.

### Setup and documentation/chart integration (2026-09-21)

Resolved the disposable checkout setup action to the official uv path and typed
Python installers. Scanner checksum/signature verification and analyzer caching
remain; coverage selects installed Clang 19 with Rust-bundled LLVM tools. All 42
focused setup-selection, action-contract and native scanner-install tests passed
(`artifacts/integration-setup-regression.log`). The exact merged action also
passed its CLI argument and official-uv contract checks.

Merged devops policy retains historical safeguards under an explicit historical
heading, ADR 591 authority, strict Sonar waiting and Python tooling requirements.
Documentation and chart workflow conflicts preserve media 60-minute job limits
and use the existing Python setup interface; both pass command/setup contracts.
Twelve integration paths are resolved, six remain. The blocked nine-job PR
proposal remains unapplied; these local checks do not establish hosted CI,
publication, application acceptance or complete integration qualification.

### Image/release and generated reference integration (2026-09-21)

Resolved the image and release workflow conflicts to existing Python tasks,
retaining media publication predicates and evidence handling. Exact merged
command/setup contracts and release eligibility checks pass; 67 focused image
and release regressions pass. No publication occurred. Full PR-dependent image
contract evaluation remains pending the unresolved PR workflow.

The actual Rust documentation indexer regenerated 554 entries. Initial API
export exposed an existing generator behavior: invalid embedded merge input
produced an empty object despite process success. Restored the reviewed media
snapshot seed, then regenerated 127 API paths including 36 media paths. Added
rv validation before and after export so malformed embedded input or empty
generator output cannot be accepted. Real Cargo regressions cover both cases;
all 18 build/export tests, Ruff checks and strict mypy (272 files) pass.
Updated structured READMEs and copied the tested change into integration.

The resolution receipt records seventeen resolved paths; only pr.yml remains
unmerged. The blocked nine-job patch remains unapplied. Generator logs and
focused results are retained under artifacts/integration-*.log. This is not
full application, coverage, native image compliance or hosted Sonar acceptance.

### Disposable single-init database tasks (2026-09-21)

Added `rv db-test-init NAME` and `rv db-test-drop NAME` with static dispatch,
injected lifecycle settings and a typed Docker/psql adapter. Reused the media
checkout's six SQL resources unchanged. The implementation preserves explicit
container/admin selection, exact configured image identity, bounded unique names,
restricted roles, transactional init/seal, owner-login removal and verified drop.
The staged initializer is independently hashed before sealing. Native testing
exposed Docker archive-copy failure into tmpfs; SQL now streams through native
`docker exec`/`tee` without a shell or host mount.

All 35 lifecycle/settings checks passed, including 11 lifecycle cases. Native
PostgreSQL verifies digest binding and runtime reads, disabled owner login,
failed-init database/role cleanup, retry after failure, existing database/role/
staging preservation and image rejection. Strict mypy passes for 276 files;
Ruff is clean. Evidence: `artifacts/database-lifecycle-validation.log` and
`artifacts/database-lifecycle-types.log`. These small SQL fixtures prove the
lifecycle protocol, not complete media initializer/application acceptance.

Updated tooling/test READMEs, the migration inventory and scoped database
instructions; copied the implementation into disposable media integration while
preserving its merged policy. No new dependencies. Rollback removes the two
new commands and adapter; existing managed-database behavior is unchanged.
No external publication or caller-owned database was modified. Full media init,
E2E routing and remaining workflow migration/qualification are still required.

### Full initializer and E2E database routing (2026-09-21)

The exact integrated init SHA-256 `440e54d8a6d8bf7aed7bb6c414ed73ea44abc07df4f1f425f1211ace86a91db9`
passed the new lifecycle on pinned PostgreSQL 16.14. Verified sealing, disabled
owner login, restricted runtime access and removal of database, roles, container
and anonymous storage. Evidence: `artifacts/full-init-lifecycle-ee352cf2ec13`.

Python E2E now selects this lifecycle in `feature-development`. Preserved the
original explicit service URL/container requirement and exact loopback binding;
only generated restricted runtime credentials reach the application. Invalid
phase configuration cannot fall back to migrations. Native lifecycle plus E2E
checks passed (41 cases); the final coordinator/phase suite passed 32 cases.
Strict mypy passes 278 files; Ruff is clean. README and scoped UI policy updated.

The actual E2E database context also passed full-init TCP runtime authentication,
exact digest read, expected baseline-table permission denial, post-denial recovery
and owned cleanup (`artifacts/e2e-full-init-256c942538a5`, `e2e-full-init.log`).
No new dependencies or caller-owned data changes. Full media application/browser
scenarios and remaining CI/Sonar migration remain unqualified. The blocked PR
workflow proposal remains unapplied.

### First media API scenario ports and live acceptance (2026-09-21)

Ported all 14 cases from media-profile-create and media-root-readiness into
Python: persisted profile creation/update, automation guards, manual discovery,
diagnostics, source preservation and persisted root readiness. Media API phase
selection is additive to foundation cases; real child pytest and coordinator
checks pass (5 focused cases). Ruff and strict mypy (282 files) pass.

The first application attempt selected the foundation production binary and
correctly failed its missing compliance-metadata check. Added explicit
`media_runner = "lib-test"` configuration, preserving the original media Rust
serving entry and per-run override without changing production startup policy.
The corrected real anonymous phase collected 49 cases: 36 passed, including all
14 new media cases, and 13 foundation cases failed. Failures include missing or
mismatched OpenAPI operations/responses, secret creation returning 500, and the
torrent detail response schema. The ordered coordinator correctly stopped before
the authenticated phase and cleaned owned resources. Do not call this full E2E
acceptance. Evidence: `artifacts/media-api-integration.log`,
`media-api-anonymous-junit.xml`, and `media-api-summary.json`.

Updated the E2E README, test README and scoped UI policy. No dependencies added;
TypeScript originals remain until remaining scenarios and full acceptance are
complete. Existing schema checks and failure ordering remain strict. Next work
must reconcile the 13 concrete integration failures and port remaining media
API/UI scenarios; the blocked PR patch is still unapplied.

### Integrated API acceptance and browser port (2026-09-21)

The real `rv` entry point completed both API authentication phases against the
owned, pinned PostgreSQL service and full media initializer: **49 + 49 passed**,
including all fourteen ported media scenarios in each phase. uv loaded the
existing test environment file through its supported `--env-file` option.
The owner receipt confirms service and anonymous-storage removal. Evidence is
retained in `artifacts/media-api-cli.log`, the two `media-api-*-junit.xml` files,
and `artifacts/media-api-cli-e9d8ae6e5515/`.

The integration OpenAPI seed required a semantic three-way merge: the earlier
media-only conflict resolution discarded foundation operations and response
schemas. Merging both branches against the tooling HEAD produced no semantic
conflicts and restored 137 paths. The successful API phases validate those
response contracts; this correction belongs to disposable integration only.

The three root-readiness UI scenarios are now Python Playwright tests, retaining
refresh/error redaction, readiness separation, desktop/mobile geometry, and
screenshots. A local DOM Range expression remains necessary for text geometry;
all orchestration is Python. Media-phase selection now adds media UI as well as
API cases while retaining the foundation suites. Full browser acceptance and
remaining media scenario ports are still pending.

### Catalog browser ports and WebKit regression (2026-09-21)

All nine original media UI scenarios are now Python Playwright scenarios. The
first eight plus thirteen foundation cases passed on Chromium and Firefox;
WebKit passed twenty and failed the empty-draft assertion after root catalog
refresh. Preserve the assertion. Evidence is retained in
`artifacts/media-catalog-browsers-before-fix/`; the earlier readiness-only run
passed all sixteen Chromium cases (`artifacts/media-readiness-browser/`).

The disposable integration now tests an explicit DOM-value reconciliation after
Yew renders root choices. This keeps the existing draft authoritative when a
browser auto-selects an inserted option, including loss-of-access drafts. The
concurrent worktree is unchanged; the reviewable integration-only patch is
`artifacts/media-profile-root-selection.patch`. It is not yet accepted evidence
of a fix and must accompany final media rebase reconciliation if validation passes.

Three additional API scenarios preserve compatibility-target rejected-write
state, all forty bounded route probes, and the two worker-owned 405 contracts.
The large combined media API lifecycle remains unported and must be reconciled
with the explicit automation-admission rules. Native API/browser acceptance is
running; no overall gate completion is claimed. Python static checking passes
for 287 source files. Four native child-pytest selection probes pass, proving that
media inclusion adds to, rather than replaces, foundation API and UI cases.

### Five-phase native media acceptance (2026-09-21)

The final run passed **52 anonymous API + 52 authenticated API + 22 Chromium +
22 Firefox + 22 WebKit scenarios**, with retries disabled. This includes all
nine ported media browser scenarios, seventeen media API scenarios, and the
foundation suites. The original empty-root draft assertion passes on WebKit
with the integration-only post-render selection correction. The real management
scenario also verifies target and policy creation through the browser.

`artifacts/media-five-phase-acceptance/` retains logs, five JUnit reports,
per-phase coverage evidence, screenshots, the runner summary, exact relevant
source hashes, and a receipt proving container and storage removal. Browser
JavaScript line coverage was not enabled in this run; this is functional E2E
evidence, not the final coverage or Sonar gate. The remaining combined API
lifecycle, workflow approval/cutover, obsolete-tool removal and full exact-revision
quality gates are still outstanding. Do not claim the whole migration complete.

### Complete API port exposes automation contract conflict (2026-09-21)

The remaining combined media API scenario is ported as five independent lifecycle
scenarios. Splitting independent subsystems prevents one failure from hiding the
others. Both authentication modes ran 57 cases: **56 passed, one failed**. The
failure retains the original requirement that a legacy path-based profile can
enable scheduling with a cadence; the current application returns 400 while the
newer profile tests require `media_profile_filesystem_identity_required`.
Positive verified-root automation remains unproven and the failing test is kept.
No gate, assertion, or capability requirement was relaxed to manufacture a pass.
The pending job steps remain in the same scenario after admission.

Target and policy authoring/validation, global retention updates, capabilities,
compliance, and portable/local YAML imports passed in both modes. Retention
restores its fetched baseline in `finally`, making the two shared-server auth
phases independent. Both profile fixture families now own nested temporary
directory contexts: pytest's retained `tmp_path` alone is not immediate cleanup.
Owned database receipts confirm storage removal for both failed runs. Logs and
JUnit reports are retained under `artifacts/media-lifecycle-anonymous/` and
`artifacts/media-lifecycle-authenticated/`. Static checks pass for 288 files.
The complete Python tooling-check gate is running at this port milestone.

### Full tooling gate and app-regression recipe restoration (2026-09-21)

At the complete media scenario-port milestone, `rv tooling-check` passed
formatting, linting, typing and **1,698 tests** in 421.87 seconds. The managed
PostgreSQL ownership test passed in this run; this does not establish a fix for
its earlier intermittent failure. The locked Python audit passed with no known
vulnerabilities. Evidence: `artifacts/tooling-check-after-media-ports.log` and
`artifacts/tooling-audit-after-media-ports.log`.

The recipe audit then identified and restored `rv ui-e2e-app-test`. Static task
dispatch uses a typed `AppRegression` enum and the existing injected Cargo adapter.
It retains default features, the two single-threaded library filters, and the
bootstrap integration binary. Missing/renamed groups cannot silently pass with
zero tests. Seven focused native Cargo fixtures passed (success plus failure and
empty-selection checks for each group); static typing passes for 289 files.
Evidence: `artifacts/app-regression-task-validation.log`. These focused results
follow the full 1,698-test run; do not describe them as one combined full-suite run.
Actual media application regression qualification is running independently with
an owned pinned database service and uv's supported environment-file handling.

### Actual app-regression fixture boundary (2026-09-21)

The real `rv ui-e2e-app-test` build succeeded and executed both launch-group
tests. `e2e_serving_entry` passed; `e2e_serving_preserves_setup_configuration`
failed with `baseline_shape_invalid`. Its existing Rust test-support helper
creates an empty database, while the feature-phase application now validates an
initialized/sealed baseline. This is an additional media integration fixture
migration requirement; the Python runner did not suppress or skip it. The task
correctly stopped before the remaining two groups. No baseline check was relaxed.
`artifacts/media-app-regressions/` retains the native output and owned-service
receipt confirming storage removal. Correct the Rust fixture's single-init
provisioning boundary before treating the actual application gate as passed.

### Explicit runtime fixtures qualify app regressions (2026-09-21)

The empty-database failure above is resolved in the disposable integration.
`TestDatabase::initialize_runtime` receives the selected checkout's initializer,
verifies the owned database, creates isolated roles, applies/seals the exact
initializer in a transaction, disables owner login and verifies runtime identity.
Raw constructors remain empty for negative baseline tests. Explicit close reports
cleanup failures; Drop also attempts cleanup and reports errors. Credential URLs
are omitted from Debug. No production baseline bypass or new dependency was added.

Media qualification passed three real PostgreSQL fixture tests, strict all-target
Clippy, and all nine `rv ui-e2e-app-test` launch/compliance/bootstrap tests. The
foundation passed its two initializer-failure/empty-input fixture tests and both
strict Clippy passes. Both owned container/storage receipts report successful
cleanup. `artifacts/runtime-fixture-qualification/` retains logs, receipts, source
hashes and the integration-only fixture patch for rebase reconciliation. The
helper README documents empty versus initialized fixtures and cleanup.

The Docker builder copies the repository including both embedded role SQL files;
`.dockerignore` does not exclude them. Full image qualification remains pending.
The scoped data instructions now cover test support, explicit runtime setup and
cleanup, and remove their stale `just lint` reference. The test caller remains
media-only; the foundation does not bundle a duplicate initializer. Rollback is
the fixture/helper change together; no deployed database is changed. This focused
evidence does not clear the API scheduling conflict, workflow approval, complete
CI/coverage/Sonar, package acceptance or obsolete-tool retirement.

### Native media contract checks and protected checkout (2026-09-21)

Actual `rv` tasks passed 95 root-contract, 80 root-catalog and 20 broker tests.
The first catalog run under `/private/tmp` failed eight descriptor tests because
the fixture uses the checkout directory and rejects writable ancestry. Directory
metadata confirmed `/private/tmp` is mode 1777; the selected worktree parent and
its ancestors are protected mode 755 with expected ownership. Git moved only the
disposable integration to
`/Users/vanna/Source/revaer-worktrees/rv-media-integration`; official
`uv sync --offline --locked --reinstall-package revaer-tooling` refreshed its
editable entry point. The same tests then passed without source or guard changes.

`artifacts/media-native-contracts/` retains both failure and passing logs. The
active media checkout remains untouched. The integration PR workflow is still
unmerged pending the separate approval. These are macOS native results; Linux
conditional filesystem/mount cases and full CI remain unproven. Future local
qualification must use protected checkout ancestry, not loosen production checks.

### Workspace gate exposes remaining positive fixtures (2026-09-21)

Combined Python/Rust formatting passed. The real `rv test` workspace run
progressed to the application library, where 336 tests passed and 16 failed.
The reported failures use empty databases although the media configuration
service now requires an initialized/sealed baseline. This is failed workspace
evidence, not successful CI. The owned PostgreSQL service was cleaned up.

Five positive bootstrap fixtures now explicitly initialize the canonical media
schema, expose the restricted runtime URL and call close after their assertions.
Provisioning errors now fail these tests instead of returning an apparent pass.
The original configuration, watcher and bind-rejection assertions are retained.
All 28 selected bootstrap tests passed against a fresh pinned service, including
these five. The remaining runtime-child, import and orchestrator fixture failures
are not fixed or claimed passing. The active media checkout is untouched.

`artifacts/workspace-bootstrap-qualification/` retains full failed workspace
output, focused passing output and the integration-only bootstrap fixture patch.
These changes belong with the media rebase, since the foundation has no canonical
single initializer. No new dependency, production bypass or gate relaxation was
introduced. Rollback is limited to these test fixture changes.

### Documentation build qualification (2026-09-21)

The default PATH exposed mdBook 0.5.4; `rv docs-build` correctly rejected it.
Selecting the already installed pins (mdBook 0.5.0, Mermaid 0.17.0) exposed a
real collision: both api/readme.md and api/index.md mapped to index.html. Renamed
the authored contract guide to api/contract.md and updated SUMMARY in both trees,
preserving both pages; API overview now names `rv api-export`. Foundation docs
then built without warnings. The media book builds but warns that its search
index is 16,678,538 bytes; its documentation gate remains unqualified until this
is addressed without hiding documentation. Logs are retained with the workspace
evidence. No search content or gate was disabled.

### Remaining application and configuration fixtures (2026-09-21)

The eleven remaining positive runtime-child, import and orchestrator fixtures now
initialize the canonical schema and explicitly close their owned database. Raw
fixture constructors and production baseline checks remain unchanged. The real
`rv test` rerun passed all 352 application-library tests, then exposed thirteen
configuration integration fixtures with the same empty-schema assumption. Those
fixtures now explicitly initialize as well, fail provisioning errors rather than
silently skipping, and close after the existing assertions. All thirteen pass
against the owned pinned PostgreSQL service.

`artifacts/workspace-bootstrap-qualification/` retains the workspace rerun,
configuration success log and `media-positive-runtime-fixtures.patch`, covering
all application/configuration fixture corrections for the media rebase. This
supersedes the prior outstanding app/import/orchestrator fixture list. A separate
whole-workspace diagnostic with Cargo's `--no-fail-fast` is running to collect
remaining failures together. It does not change the fail-fast `rv test` contract
and is not yet passing workspace evidence. No new dependency or production
behavior change was introduced; rollback is limited to the test fixture patch.

### Grouped data/runtime/filesystem qualification (2026-09-21)

The complete diagnostic finished with five failing targets. Positive fixture
setup has now been corrected in data configuration/runtime integration tests,
the runtime-store helper, the filesystem runtime helper and the media SQL test
helper. Provisioning errors propagate; successful tests explicitly close owned
resources. The media SQL tests retain their original administrative connection
for deliberate trigger/row mutation assertions, separately verifying the sealed
baseline using the restricted runtime role before opening that admin pool.

The grouped rerun passed all four data configuration tests, three data runtime
tests, 74 filesystem tests and ten runtime tests. The restricted-baseline pool
test that previously timed out passed unchanged. The data library remains
failing: concurrent schema creation exhausted PostgreSQL shared memory in the
media fixtures and a schema test. A serial diagnostic of the same media tests
is running; no memory limit, statement timeout, safety check or assertion has
been relaxed. This is not complete workspace acceptance.

The full diagnostic, grouped rerun and `media-data-runtime-fixtures.patch` are
retained under `artifacts/workspace-bootstrap-qualification/`. These are media
integration fixture corrections, not changes to the active media checkout.

The serial media diagnostic passed all sixteen selected tests (fourteen database
scenarios plus two identity tests) in 31.59 seconds and confirmed owned storage
removal. The complete standard `rv test` gate is now running with the native
`RUST_TEST_THREADS=1` scheduling setting. Internal concurrency tests, all feature
selection and statement timeouts are unchanged. This avoids concurrent whole-
schema initialization on the default-sized disposable server; the full result
is still pending. Evidence: `media-fixtures-serial.log` in the same directory.

### SQL fixture policy correction (2026-09-21)

The foundation policy gate found inline DDL in the new fixture rollback test.
Moved that intentional failing initializer to
`scripts/tests/database-runtime-fixture-failing-init.sql` and embedded it with
include_str!, preserving the failure/cleanup assertions. The two focused native
fixture tests and both strict Clippy passes succeed. The policy rerun now reports
90 findings, all confined to the still-unconverted PR and Sonar workflows. The
SQL correction is in the foundation; integration synchronization is pending the
current full workspace run so its source is not changed during validation.

Logs are retained under `artifacts/workspace-bootstrap-qualification/`. An
independent existing LanguageLint component run is underway in the foundation;
it cannot establish a passing `rv lint` while the workflow policy gate fails.
No policy exception or workflow rejection was bypassed.

The foundation LanguageLint component completed successfully: Python lint/type
checks and both all-feature Rust Clippy passes. Evidence is retained as
`foundation-language-lint.log`. The separate workflow policy gate remains failing.

### Full host workspace, dependencies and chart gates (2026-09-21)

The complete media `rv test` command passed on macOS with `RUST_TEST_THREADS=1`,
all workspace features and the owned pinned PostgreSQL service. The receipt
confirms service/storage removal. Cargo retained the existing ignored
`verify_prepared_fixture_suite` scenario, which requires the separate prepared
media-conversion gate; Linux-only tests are outside this host run. This establishes
the standard host workspace test command, not full CI or all release gates.

Both foundation and media `rv audit` and `rv deny` passed: Rust advisory checks
covered 436 and 474 dependencies respectively; locked Python all-group audits
found no known vulnerabilities; licenses, sources and bans passed. Both actual
`rv helm-lint` commands pass. Media qualification includes six annotation, 102
compliance and 47 package/version tests, then strict native lint and packaging.

The media chart originally exposed an invalid warning-test control, missing its
required image/compliance bindings. The control now receives synthetic values
outside the chart; it must still succeed without strict mode, emit its intentional
warning, and fail packaging under strict mode. Foundation's focused case passes.
Published defaults and production chart validation are unchanged. No dependency
was added. Logs and the workspace receipt are in `artifacts/integration-gates/`.

After the full workspace run, the already verified failing-initializer SQL fixture
was synchronized into integration, eliminating embedded test DDL there too. The
combined strict LanguageLint check is running; focused integration fixture
revalidation after that source-only SQL relocation remains pending. Workflow
policy, media scheduling contract, image/conversion/coverage/Sonar and final
exact-revision qualification remain outstanding.

The combined LanguageLint component now passes Python checks and both strict
Rust Clippy passes. Two existing pure directory identity helpers needed `const`;
the initialized compliance test needed its parent-owned setup/cleanup separated
from child-serving assertions to stay below the function-size limit. No lint
suppression was added. `media-lint-followup.patch` and all lint attempts are
retained in `artifacts/integration-gates/`. Focused initialized-fixture and
runtime-child tests are running after these final small changes; the earlier
full workspace pass must not be misrepresented as covering later source edits.

Post-lint focused verification passed all three initialized-fixture tests and
all fourteen runtime tests, including the extracted compliance child boundary.
Owned service/storage removal is confirmed in the retained
`artifacts/integration-gates/final-fixture-checks.log`. This covers the final SQL
relocation and test refactor after the earlier full host workspace pass.

### Actual prepared-media conversion qualification (2026-09-21)

`rv download-test-fixtures` verified all 22 locked sources; generation produced
the declared derivatives. `rv test-media-conversion` verified all 30 reviewed
probe snapshots, retaining the existing exact ADR578 F1 exception, and passed
all six actual Rust integration tests with zero ignored tests. The fresh report
contains 30 verified fixtures and 30 pipeline actions, including eight video and
six audio transcodes, three metadata checks and zero pipeline/suite failures.
This closes the prepared-fixture scenario omitted by normal workspace tests on
this host; it does not establish Linux/image deployment qualification.

`artifacts/media-conversion-qualification/` retains acquisition/generation/run
logs and both fresh reports. Probe diagnostics remain in the integration's
`target/media-conversion-report.md.preparation.probe-evidence/`. No snapshot,
exception, expected action count or gate was changed. The full Rust/native
coverage command is now running with an owned pinned database and unchanged
per-crate thresholds.

### Shared debugger manifest support (2026-09-21)

The first actual local image build reached `rv container-build` but rejected the
media manifest's eleven PostgreSQL native-debugger fields. BuildInputs now
recognizes exactly that optional block and requires all its fields when any are
present. It validates seven artifact hashes, the immutable debugger image ID,
exact Alpine/package versions and a credential/query/fragment-free HTTPS musl
source URL, including valid ports. Unknown fields still fail. Container builds
do not install debugger tools or fetch this source as a side effect.

All 70 build-input tests pass, including missing-field, unpinned-value, malformed
URL and existing unknown-key mutations; formatting, lint and typing pass. The
image README and scoped DevOps instructions describe the contract. Only standard
library URL parsing was added; no dependency changed. The parser is synchronized
into disposable integration and the dedicated local image build is retrying.
Logs are retained as `native-build-inputs-tests.log` and
`image-native-inputs-rejection.log` in `artifacts/integration-gates/`. The dedicated
`rv-tooling-qualification` builder loads a separately named local image and does
not publish. This is not yet image/compliance acceptance.

### Local ARM64 application image and configured scan (2026-09-21)

The integrated Dockerfile built and loaded `revaer-rv-qualification:local-media`
for Linux ARM64 through the existing typed DockerBuild task. The retained image
manifest digest is
`sha256:cb888b4793559df7a2ca6a8d60d6e9768ee203d71563fd4c6a0868ca9d7ee0b2`.
DockerScan passed with its unchanged configured severity/scanner policy and empty
ignore file; retained JSON contains zero reported vulnerability, misconfiguration
or secret findings. Alpine 3.23.5 and 203 OS packages were detected.

Build/scan logs, digest/provenance metadata and Trivy JSON are retained under
`artifacts/integration-gates/`. The image was not published. This is a local
working-tree qualification, not a clean exact-revision release build, AMD64
qualification, signed publication or final-image compliance attestation. Those
gates remain outstanding. The foundation instruction-drift task also passed.
Rust/native coverage continues running and has not yet produced a passing gate.

### Native event subscription race (2026-09-21)

The full coverage attempt terminated before report generation because
`native_alerts_and_rate_limits_smoke` missed `TorrentAdded`. Inspection confirmed
that `EventBus::subscribe(None)` receives live events without replay, while the
test subscribed only after adding the torrent and updating limits and deadlines.
The test now subscribes before issuing commands in both isolated worktrees; its
event assertion and timeout remain unchanged. All three native integration tests
passed with `REVAER_NATIVE_IT=1`, all features, warnings denied and serial test
execution. Evidence: `artifacts/integration-gates/native-event-check.log`.
The equivalent instrumented check also passed all three tests; its log is retained
as `artifacts/integration-gates/native-event-instrumented.log`. Full workspace
coverage has been restarted and remains unproven.
No production event behavior, coverage threshold or workflow criteria changed.

### Local image inventory boundary (2026-09-21)

The existing ImageInventory task rejects the local Docker tag because package
inventory requires a remote immutable image. The built image has no RepoDigests
entry. This local build therefore cannot establish the final digest-bound
compliance bundle. The guard remains unchanged; no image was published.
The rejection is retained in
`artifacts/integration-gates/local-image-inventory-rejection.log`.

### Current Python tooling coverage (2026-09-21)

`uv run --offline --locked rv tooling-cov` passed all 1,737 tests in
500.34 seconds. The unchanged 90% gate passed: the complete authored Python
report measured 94.98%, and the separate tooling/launcher implementation report
rounded to 93% combined line/branch coverage (9,756 statements, 512 missed;
2,806 branches, 288 partial). XML and LCOV reports remain in `coverage/`;
the command log is retained as `artifacts/integration-gates/tooling-coverage-final.log`.
This verifies the foundation tooling suite; Rust/native workspace coverage and
whole-project acceptance remain outstanding.

### Workspace coverage threshold result (2026-09-21)

The corrected native test passed during the full workspace coverage run. All
workspace tests completed and JSON, LCOV, text and HTML reports were generated.
Seventeen crates passed the unchanged 90% Rust line gate; `revaer-app` failed at
17,818/20,070 lines (88.78%). The owned database was removed. Retained evidence:
`artifacts/integration-gates/media-coverage-retry.log` and
`artifacts/integration-gates/media-app-coverage.json`.

Inspection found the indexer-service fixture still opened a raw database and 27
callers silently returned success when service setup failed. In the disposable
integration tree, the fixture now invokes the existing sealed runtime initializer
and those callers propagate setup errors. Focused verification is running; this
is not yet a claim that the application coverage shortfall is resolved.

The focused indexer run completed: 45 passed and four failed, with owned database
cleanup confirmed. Three failures are direct fixture seed writes through the
restricted runtime connection (indexer_instance, policy_set, rate_limit_policy).
The fourth exposed a database procedure error: ambiguous
`canonical_torrent_public_id`. Evidence is retained in
`artifacts/integration-gates/indexer-fixture-checks.log`. The fixture correction
remains integration-only while these failures are resolved; no permission or
coverage policy was relaxed.

### Indexer fixture and ingestion corrections (2026-09-21)

The integration fixture now keeps an administrative connection only for explicit
seed writes and internal-ID reads; application calls retain the separately
verified runtime role. The ingestion workflow exposed ambiguous table columns
in `search_result_ingest_v1`: canonical/source public IDs conflict with output
parameters, and the context-score lookup conflicts with its local score variable.
Those SELECT references now identify their tables explicitly; no compiler
name-resolution directive, grant, migration or coverage gate changed.

The full affected suite reached 48 passing tests with one fixture-read failure.
After separating that final read, the remaining case passed on its own. Logs
are retained under `artifacts/integration-gates/revaer-indexer-*.log`. These
corrections are integration-only pending final lint, SQL failure/rollback
qualification and the refreshed workspace coverage gate.

### Fresh initializer ingestion qualification (2026-09-21)

All ten focused ingestion tests passed against the current sealed initializer,
including invalid-request/identity/duplicate-attribute rejection, monotonic
observations, identity conflict preservation, title/size fallback, rollup median,
append-only sealed pages and dropped-source filtering. Setup errors now fail
these tests instead of returning success. A new test ingests through the
restricted runtime role inside an explicit transaction, rolls back, and verifies
canonical/source/observation counts are unchanged. Duplicate-attribute rejection
also verifies unchanged counts. Seed helpers retain administrative access only
for fixture setup. The owned service/storage was removed; evidence is retained
at `artifacts/integration-gates/ingestion-init-check.log`.
Strict application lint passed before the data-test addition; final two-crate
lint and refreshed full workspace coverage remain to be checked.

Final strict all-feature/all-target Clippy passed for both revaer-app and
revaer-data after renaming the now-used test database ownership field. The full
workspace coverage gate has been restarted with the unchanged threshold.

### Full workspace coverage passed (2026-09-21)

The complete `rv cov` run passed all 18 per-crate 90% Rust line gates. Application
coverage increased from 88.78% to 91.25% (18,313/20,070 lines) after restoring
real execution of the indexer tests. All 352 application tests passed in this
instrumented run. Data coverage is 97.67%; native C++ `ffi/session.cpp` retains
positive measured coverage of 1,363/2,358 lines (57.80%) without a new native
threshold or exclusion. JSON, LCOV, text and HTML reports were generated.
The owned PostgreSQL service and storage were removed.

Evidence: `artifacts/integration-gates/media-coverage-passed.log` and
`native-coverage-passed.json`; full reports remain in the integration worktree's
`coverage/`. The exact five-file integration correction is retained in
`artifacts/integration-gates/media-indexer-initializer.patch`. Both all-feature
and no-default-feature strict Clippy checks passed for the affected crates.
This is a passing disposable integration coverage milestone, not complete CI,
E2E, Sonar, clean-release or final-image compliance acceptance.

### Tooling merge and full media restack (2026-10-02)

The operator authorized merging this foundation, then rebasing the full media
PR stack and pushed operator checkpoint `71596802`. The paused shared checkout
contains unrelated conflicts and remains untouched. Recovery snapshots preserve
all four worktrees, their indexes and changed/untracked files. The dependency
plan records all 104 open media PRs and their original remote heads; eight
prerequisite heads are already included in this foundation. PR 98 has a base
that is not an ancestor of its head and needs an explicit merge-base comparison.

The PR and main Sonar workflows now invoke the locked Python executor. Native
coverage and scanning share one checkout; Python and bootstrap coverage have
explicit producers. UI aggregation retains completed shard selections and raw
browser evidence. Required context names and gate conditions remain intact.
The existing media required-check snapshot is now present in the foundation.
Sonar inventories include the added tooling and fixtures without exclusions.

Validation so far: 135 focused workflow/setup/contract tests passed and `rv
policy` passed. Full tooling checks and `rv ci` are running; merge, restack,
complete E2E and hosted scanner acceptance are not yet claimed. No dependencies
or quality-gate exceptions were added. Workflow logs and the branch plan are
retained under `artifacts/rebase-20261002/`.

Stale-policy review: reviewed root policy, DevOps and Sonar instructions. Updated
Sonar execution/coverage guidance to Python and the retained root bootstrap, and
removed the obsolete recommendation to rely on decoration instead of scanner
waiting. Recovery refs and worktree snapshots support rollback before any
remote branch update; remote rewrites must use their recorded expected heads.

The first current `rv ci` run reached the dependency audit after formatting,
lint, Helm, drift, assets and unused-dependency checks passed. The audit found
three advisories in urllib3 2.7.0. A targeted uv lock update to 2.8.0 changed no
other package; Rust and Python audits then passed with no known vulnerabilities.
Full CI was restarted against the corrected lockfile. Root and developer
instructions now identify `rv` as the approved canonical executor; the root
README documents its actual setup and formatting behavior.

The full tooling check passed 1,736 tests and failed its real container build
fixture because Alpine replaced the pinned OpenSSL 3.5.8-r0 packages. Verified
the repository package indexes inside both pinned base images and updated only
the builder/runtime OpenSSL pins to 3.5.9-r0. The fixture is being rerun unchanged;
no package requirement, checksum or image boundary was relaxed.

Hosted validation of PR 211 caught two checkout differences: the declared SPDX
inventory still named the old OpenSSL pin, and Helm's registry regressions lacked
ORAS. Downloaded both official 3.5.9-r0 APK archives and verified their signatures
with native `apk verify` in each architecture's pinned Alpine image before
recording the real SHA-256 values in the declaration. The local container fixture
then passed unchanged. Added ORAS to Helm setup and all native tooling-test
prerequisites to scanner jobs, which execute the complete Python suite.
This updates the declared package inventory; it is not final-image compliance
evidence. No scan, signature check, or gate was bypassed.
