# Testing rv

The tooling suite checks behavior at three boundaries: Python coordination,
real local tools, and publication protocols. It runs with the locked environment:

```console
rv tooling-check
rv tooling-cov
rv tooling-audit
```

Run `./setup.sh` first. The full suite needs the native development tools as well
as Python; `--profile python` only bootstraps the Python environment and launcher.
The native setup tests inject the installer boundary to check ordering, pins,
profiles, and failure handling. Version probes use the real tools; installation
requests are recorded. Running tests must not change global native installations.

## Local test configuration

Local `.env` files may contain arbitrary testing passwords. The operator-approved
pre-read scan exception is documented in [AGENTS.md](../../AGENTS.md#pre-read-secret-scans).
When scanning files before inspection, enumerate paths and omit files named
`.env`; do not pass their containing directory to a recursive scan.

Test configuration also contains dummy PostgreSQL URLs for local E2E databases
and Helm rendering. These source files remain scanned. The Helm rendering value
does not connect to a server. Keep test values distinguishable from credentials
used by release, registry, or other external integrations. The local `.env`
exception does not change CI's Sonar scope or quality gates.

## What the tests establish

| Area | Evidence |
| --- | --- |
| Bootstrap and launcher | Actual uv installation in temporary tool directories, independent worktrees, locked execution, path handling, collisions, and source-checkout removal. |
| Dependency groups | A fresh locked core install runs the actual CLI and exports every audit group. An official uv/Alpine container downloads managed musl Python and invokes the installed entry point without browser or release dependencies. |
| Root setup script | The unchanged script runs both directly and under actual kcov. Official unmanaged installation, explicit directory precedence, foreign environment selectors and stale-lock rejection preserve the fixture boundaries and lockfile. |
| Native coverage tool | Real CMake builds a small C++ fixture through HTTPS/hash verification; cached files and modes are checked, modified installs are rebuilt, and build failure preserves the prior executable. This fixture supplies version text; actual kcov measures the root-script cases. |
| Processes | Literal arguments, separate stdout/stderr, redaction, stdin, exit codes, timeouts, interruption, failed decoding, descendant cleanup, and termination escalation. |
| Policy | Real Git changes and fixtures for prohibited Rust constructs, unsafe workflow expressions, duplicate/cyclic YAML, and instruction drift. |
| Build and docs | A small Rust 2024 workspace built with Cargo, production Clippy checks, generated assets, source-bound release files, configured target directories, and real mdBook output. |
| Rust tests and dependencies | Real Cargo tests observe package/feature selection and fail when requested; pinned cargo-udeps detects an intentionally unused local dependency and retains compiler evidence. |
| Rust/native coverage | A Rust workspace with a real C++ bridge produces line records and HTML; an uncovered crate fails its own 90% gate, compiler warnings fail collection, and independent Python reports survive. |
| Database lifecycle | Real Docker/PostgreSQL/SQLx/libpq checks prove ownership, idempotent start/seed, explicit reset, data-preserving container recreation, checksums, and rollback. |
| Media scenario selection | Real child pytest runs prove media API paths are additive to foundation paths; coordinator cases preserve runtime-role URLs and cleanup ordering. Scenario collection is not live application acceptance. |
| Single-init test lifecycle | Native PostgreSQL verifies initializer digest sealing, restricted runtime reads, disabled owner login, failed-init cleanup, collision preservation, image mismatch, explicit credentials and CLI dispatch. Small SQL fixtures prove lifecycle behavior; full media initializer acceptance remains separate. |
| Database transition | PostgreSQL scanner boundaries, frozen hashes/phases, exact routine deltas and private proof resource ownership. Failure injection covers partial startup, migration retries, schema drift and cleanup; native PostgreSQL verifies transactional rollback and warning rejection. The separate 167-migration replay reproduced all original candidate artifacts exactly. |
| Development | Real Cargo/Trunk serving, native watchfiles/Git events, process creation identity, token-bound orphan recovery, checkout isolation, TERM/KILL escalation and failed-build recovery. The process/watch contracts also run in a disposable Linux container. |
| Charts | The real chart packaged by Helm, temporary GPG signing keys, provenance verification, tamper rejection, and preservation of previous output on failure. |
| Releases | The real Python Semantic Release and GitHub CLIs, temporary Git histories and bare remotes, a local API service, upload failures, conflicting assets, and recovery. |
| Browser fixtures | Real Chromium, Firefox and WebKit contexts verify storage, retries, traces/screenshots/videos, closed-page recording, and route accumulation. Parallel workers, empty shards, setup failures and timeouts have real pytest regression checks. |
| E2E coordination | Injected failures verify port ownership, migration/startup/phase failures, interruption, cleanup ordering, current-run summaries, and successful recovery after failure. |
| API export | Real Cargo generation and release packaging require OpenAPI metadata and nonempty paths. Invalid embedded JSON fails before compilation; a generator that exits successfully with an empty document still fails validation. |
| Validation composition | Database selection and operation locks span every gate; failure prevents the later release build. The lock command runs the real uv resolver. |
| Native analyzer inputs | Real Cargo build scripts regenerate missing compilation databases on repeat runs; invalid, foreign, missing, and stale inputs fail. |
| Container images | Real Buildx load and multi-platform OCI exports retain platform metadata without changing the selected builder. Trivy detects a disposable test key and retains its failing report. |
| Image compliance | v2 bundle generation matches the legacy metadata and five artifact contents in a controlled comparison. Mutated inventories, image bindings, hashes, files and SARIF findings fail. Fixture inventories do not establish actual image compliance. |
| Chart registry | A pinned Distribution registry requires TLS and authentication; downloaded chart, provenance, and metadata bytes match their packaged source. Bad credentials and absent certificate trust fail. |
| Shard evidence | Completion summaries and exact selections reject missing phases, conflicting worker collections, duplicate scenarios, setup-only API evidence and unexpected empty suites. |
| Scanner policy | Structured workflow contracts, duplicate/aliased YAML, exact Sonar properties and complete source classifications fail closed. The fixture records its media source revision. |
| Python analysis coverage | Actual Coverage.py databases merge into full checkout-relative paths; never-imported files remain uncovered. Partial E2E phases and corrupt databases cannot reuse previous reports. |
| Browser source coverage | Real Chromium executes classic scripts and ES modules with Unicode and CRLF, repeated navigation and early closure. Unused modules remain zero without fetching imports. Changed sources, invalid counters, partial reports and incomplete runs fail; capture errors retain traces/videos. |
| Scanner installation | Real HTTPS and GPG fixtures verify archive bytes, primary signing identity, safe ZIP extraction, cache revalidation, and repair of modified extracted files. |
| Scanner execution and results | An explicitly simulated child process tests one scanner invocation, warning failures, redaction, and full binary report retention. Real local HTTP tests exact analysis IDs, retry limits, PR/main scope, blocking hotspots, and retained failure evidence. |
| Scanner preparation | Real shallow Git clones preserve working changes while enforcing exact event ancestry. Input gates require retained native headers and complete coverage; source cleanup refuses tracked files and archives browser reports. |

Fixtures and wrapper checks do not establish that the Revaer application builds
or that its browser scenarios pass. Those are separate acceptance requirements in
[the migration inventory](../migration.md).

On 2026-09-15, the complete tooling suite passed **1422 tests** with **95.04%**
overall coverage and **94%** implementation/launcher coverage on the shared
`ec8adc14` baseline plus the recorded lockfile updates. Ruff and strict mypy also
passed. The database/guard-stage result is retained in
`artifacts/database-guards-tooling-coverage.log`. The official macOS AArch64 SonarScanner archive passed actual install and
repeat-install checks. These installation checks do not establish an analysis
result; see the [Sonar guide](../src/revaer_tooling/sonar/README.md).
That complete-suite result includes native Dockerfile/Linux apt fixtures,
dependency groups, post-merge/tag workflow contracts, chart-version resolution,
docs deployment, compliance, and the earlier browser/bootstrap regressions.
The latest run includes the complete pristine catalog mutation suite and native
reference validation, alongside candidate replay, development watching/recovery,
the media port, exact process-byte preservation and official uv child-limit changes.
The earlier development run remains in `artifacts/tooling-development-coverage.log`.
The prior 1089-test media run remains
in `artifacts/tooling-media-recovery-coverage.log`. An earlier run failed on
Alpine DNS resolution and a Linux environment installed over the host file share;
`artifacts/tooling-container-coverage.log` retains both failures. The fixture now
uses uv's supported container-local environment directory, and the task-owned
VM uses explicit DNS resolvers. Both native cases passed before the complete rerun.

Preserving native CRLF also exposed a release parser dependency on normalized
HTTP headers. The first full media run retained that failure in
`artifacts/tooling-media-coverage.log`; 62 focused checks passed after fixing the
HTTP parser, then the full run above passed. The Linux source kcov install/repeat
and four measured bootstrap cases also passed with uv's official child-limit
option; see `artifacts/kcov-linux-uv/`.

The real foundation documentation build also passed without warnings and indexed
355 entries. `rv docs-prepare` produced the deployment payload without publishing
Pages; see `artifacts/docs-build-validation.log` and
`artifacts/docs-deployment-validation.log`. These records bind to the source at
the time of the run; later documentation edits require rebuilding.

## Fixture ownership

The [media tests](../src/revaer_tooling/media/README.md#validation-scope) use
recorded integration manifests and snapshots, synthetic source/probe doubles for
adversarial cases, and actual curl/FFmpeg/Cargo checks. They require the installed
FFmpeg codec set used by the reviewed recipes. Native generation downloads the
existing SHA-locked Big Buck Bunny sample into a fixture-owned directory.
The source/probe suites passed 49 and 70 cases respectively; combined media,
process, Cargo and dispatch validation passed 183 checks before the last boundary
regressions were added. The expanded 22-source/30-snapshot CLI proof is retained
separately in `artifacts/media-fixture-proof/` and is not application conversion
acceptance. Its complete source and implementation identities are recorded.

The container construction fixture builds the unchanged repository Dockerfile
with a small Rust 2024 application and the actual locked core Python package.
Native arm64 validation passed with exact runtime package versions, nonroot
ownership, exec-form health configuration, and no Python tooling files in the
final image. Evidence lives in `artifacts/container-dockerfile-proof/`. This
establishes the Dockerfile/uv boundary, not complete Revaer image acceptance.
Unit checks separately cover package/build failures, inconsistent pins, missing
compiler output, account creation failures and repeat runtime preparation.

Temporary Git repositories, tool installations, API data, and signing material
belong to the test that creates them. Tests must not rewrite the shared Revaer
checkout, publish a GitHub release, upload to a real registry, or alter system
certificate trust. Signing tests stop their own GPG agent before removing its
home. Process tests terminate only their owned process groups.
GnuPG homes use private short paths under `/tmp`, because Unix socket paths can
be shorter than the configured temporary directory. Chart tests also exercise a
deliberately long temporary directory for ordinary staging files.
Image tests create uniquely named builders and image tags, then remove only those
resources. Registry tests require OpenSSL and `htpasswd`; registry storage is a
temporary memory mount and certificate trust belongs to each client invocation.
Database tests create PostgreSQL 16 containers with temporary memory or owned
host-backed storage, ephemeral passwords, and temporary loopback ports. Cleanup
uses exact container IDs. These tests require a running Docker daemon plus
`psql` and `pg_isready`; they never use the caller's database URL. Managed tests
also exercise directory names containing spaces/commas, concurrent-operation
locks, and a deliberately mismatched PostgreSQL identity.

Coverage tests require compatible Clang C/C++ compilers and the selected Rust
toolchain's `llvm-tools-preview`. They honor explicit `CC`/`CXX`, use Clang 19
from PATH when available, and select the installed Homebrew LLVM 19 formula on
Apple Silicon when no override is supplied. They install nothing. The fixture
build script consumes the target-specific instrumentation flags supplied by
cargo-llvm-cov and links a real native object into its Rust test binary.

Release tests run PSR against local HTTPS using a temporary certificate. GitHub
CLI uses its documented `http_unix_socket` setting in a temporary configuration,
because its macOS TLS implementation uses system trust. Both clients reach the
same local API fixture. Git pushes are redirected to the fixture's bare remote.
The fixture deliberately returns failures; those responses are evidence for the
error-path tests, not ignored production failures.

## Coverage and failure handling

`rv tooling-cov` measures real Python lines and branches, including authored
tooling tests, and requires at least 90%. It additionally checks the implementation
and launcher at 90% independently, so adding covered test scaffolding cannot
dilute the runtime gate. Complete XML and LCOV reports beneath `coverage/` retain
the test sources, including when the floor fails. Do not lower the floor, omit source, add suppression comments, or replace
behavior checks with assertions copied from the implementation.

When changing a tool adapter, test the observable result and failure behavior.
Use an injected runner where a real external operation is unsuitable. Explain the
boundary that is simulated, and keep a real-tool check where it establishes
compatibility that a simulated process cannot prove.

The E2E support tests use real HTTP for path encoding, JSON schema references,
visible redirects, triggered SSE streams, and credential-safe failures. A compiled
Rust fixture proves exact Cargo binary/library-test selection, target paths with
spaces, and listener ownership through lsof. These checks cover the runner's
boundaries; the complete Revaer browser scenarios remain a separate acceptance gate.

The dotenv regression executes uv itself through rv's CLI reentry. It checks
quoted values, interpolation, caller-environment precedence, and an unchanged
lockfile. No test substitutes a custom dotenv parser for uv's implementation.


## Managed PostgreSQL startup failures

The managed-database fixture captures bounded Docker state and server logs before
removing its owned container. Pytest displays this evidence on failure. It omits
container configuration/environment and redacts generated password values. Read
these logs before retrying: a readiness timeout may mean the server has already
exited, rather than needing a longer startup timeout. A passing retry alone does
not explain or resolve an earlier lifecycle failure.
