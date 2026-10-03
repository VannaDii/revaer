# Sonar analysis and evidence

This package preserves Revaer's existing scanner and coverage requirements in
Python. The [task layer](../tasks/sonar.py) coordinates it through the `rv` command
registry. The [migration inventory](../../../migration.md) records which commands
have passed fixture, application, and integration checks. The workflow cutover
and complete scanner acceptance are still in progress.

## Ownership and architecture

| Component | Responsibility |
| --- | --- |
| `settings.py` | Parse existing environment overrides once at CLI startup; keep tokens out of representations. |
| `inputs.py` | Validate actual coverage reports, complete Python filenames, and retained native measurements. |
| `results.py` | Bind a scanner task to its completed analysis and evaluate published metrics, gate, issues, and hotspots. |
| `install.py` | Install the official native scanner archive using pinned hashes and a pinned signing key. |
| `external/sonar.py` | Construct the scanner invocation and bounded, authenticated Sonar API requests. |
| `tasks/sonar.py` | Own artifact invalidation, locks, source preparation, and complete report retention. |

No module changes Sonar server criteria. `sonar-project.properties` remains the
single scanner-criteria source. Workflow validation uses the
[policy package](../policy/README.md) to reject missing inputs, narrowed scope,
duplicate scanners, and discarded evidence. Scanner commands accept no arbitrary
arguments; inherited parameter dictionaries and JVM property overrides fail.

## Install and select the scanner

```console
rv setup --profile python --no-launcher --sonar-scanner
```

uv continues to own every Python installation and environment. SonarScanner is
a native upstream tool: its [official ZIP distribution](https://docs.sonarsource.com/sonarqube-cloud/analyzing-source-code/scanners/sonarscanner-cli)
is installed separately. The reviewed version, four platform archive digests,
and primary signing fingerprint are in `tools/versions.toml`. The committed
public key was carried forward unchanged from the media integration.

The default install location is `~/.local/revaer/sonar-scanner/VERSION/PLATFORM`.
CLI startup selects the installed version required by the current checkout,
then falls back to `sonar-scanner` on PATH. `SONAR_SCANNER_COMMAND` explicitly
selects an executable, which must still pass the version check.

`SONAR_SCANNER_INSTALL_ROOT` selects an absolute install root.
`SONAR_SCANNER_BINARIES_URL` supports an HTTPS distribution mirror; redirects
must retain HTTPS, and the checksum and signer still must match the reviewed
pins. `SONAR_SCANNER_VERSION` must equal the pin. `SONAR_SCANNER_FLAVOR` may select
one of the four reviewed Linux/macOS architectures.

Every setup verifies cached archive bytes and their detached signature, then
recreates the extracted tree. GnuPG uses a private, temporary home with the
committed fingerprint as its explicit trust anchor. No live keyserver or user
keyring is used. A valid signature from another primary key fails. Traversal,
links, special files, duplicate archive entries, and oversized downloads fail
before an executable is published. Failed installation preserves the previous
installed tree and retains the failing cached bytes for diagnosis.

## Analysis sequence

The workflow must keep these operations in the same checkout and run attempt.
Coverage producer migrations marked pending in the inventory must be finished
before this sequence replaces the existing workflow.

1. Run Rust/native coverage, Python tooling coverage, and the complete API/browser
   suite with `E2E_BROWSER_COVERAGE=1`. Each producer invalidates its previous
   success artifacts first.
2. Run `rv js-coverage-merge` and `rv python-coverage-merge` with the same E2E settings.
   Collect the root `setup.sh` with `rv script-coverage`; its
   [documented legacy-policy conflict](../bootstrap/README.md#tests-and-coverage-evidence)
   must be resolved before workflow cutover.
3. Run `rv sonar-compile-db`, then `rv sonar-verify-inputs`.
4. Retain coverage artifacts, then run `rv sonar-prepare-sources`. This refuses
   to remove tracked files and archives browser reports before cleaning generated
   mirrors from source directories.
5. Set `SONAR_BASE_SHA`, `SONAR_BASE_REF`, and `SONAR_HEAD_SHA` from the event and
   run `rv sonar-prepare-scm`. It fetches complete history and requires the exact
   event base to be an ancestor of the exact checked-out head. It does not move
   the worktree, index, or branch. An empty shallow marker is removed; nonempty
   shallow history is never discarded.
6. Supply `SONAR_TOKEN` and run `rv sonar-scan` once. Any scanner `WARN`, absent
   task identifier, or missing full submitted report fails.
7. Run `rv sonar-package-report`, then `rv sonar-verify-result`. Retain the log,
   report-task file, complete report archive, SCM evidence, and API responses
   even when verification fails.

## Python coverage paths and completeness

`rv tooling-cov` keeps the independent 90% implementation/launcher floor and the
complete tooling-test report. `rv python-coverage-merge` combines its `.coverage`
database with the named phases of the completed, unsharded E2E run. Use the same
E2E environment settings for both commands. It does not glob old phase files or
delete original coverage data. Corrupt input databases fail instead of being
silently skipped by Coverage.py.
The merge holds both the E2E operation lock and the tooling coverage lock.
Tooling collection uses the same latter lock, so neither producer can replace
inputs mid-merge. Failed merges invalidate partially generated XML/LCOV outputs.

[Coverage.py's supported merge and report commands](https://coverage.readthedocs.io/en/latest/commands/cmd_combine.html)
produce `coverage/python.xml` and `coverage/python.lcov`. The reporting-only
configuration in `tools/coverage-report.toml` uses one checkout root so files
with the same basename retain distinct paths. Every authored Python file from
the Git/nonignored working-file inventory is passed to the reporter. Files
never imported during tests remain present with zero execution. No source or
coverage exclusion is added, and no hit count is rewritten.

The input gate requires positive Rust and Python execution, at least 1,000
browser line records with covered and uncovered lines, positive native bootstrap
execution with at most one uncovered line, and the bridge's compilation database and staged CXX
headers. Native `session.cpp` coverage is mandatory on CI Linux. If present on
another host, it must also contain executed lines.

The browser producer uses Python Tree-sitter parsing for a complete source
inventory and Chromium's native V8 counters for execution; its
[measurement and page-lifecycle contract](../e2e/README.md#javascript-source-coverage)
describes source identity, unused modules, and failure retention. Real classic
and module browser fixtures pass. Every current JavaScript source also passed
the parser inventory check; full application coverage acceptance is still pending.

## Results and failure recovery

Result verification resolves `ceTaskId` to its exact analysis ID and checks that
analysis's quality gate. It requires positive coverage and lines-to-cover,
`ignoredConditions=false`, zero unresolved issues, and zero current hotspots.
No hotspot status filter hides reviewed or acknowledged hotspots.

Set `SONAR_PULL_REQUEST` for PR/new-code scope; its absence checks the main
backlog. Authentication prefers `SONAR_AUTH_TOKEN` for API reads, then
`SONAR_TOKEN`. API retries cover connection failures, HTTP 429, and HTTP 5xx;
result retries cover publication lag. Retry counts and delays retain their
`SONAR_API_RETRY_*` and `SONAR_RESULT_RETRY_*` overrides.

The last attempt's complete or partial API responses are private files under
`artifacts/sonar/api`. Previous success responses are invalidated before a new
attempt. Re-running packaging and result verification reuses the recorded task;
it does not create a second scan.

## Verification and limits

Real local fixtures exercise TLS, GPG signatures, archive extraction, shallow Git
clones, Coverage.py data, task retries, HTTP errors, and complete binary report
retention. The scanner fixture only models process output; it does not analyze
code. The official macOS AArch64 8.1.0.6389 archive also passed download, digest,
signature, extraction, and version verification on 2026-09-15.

These results do not establish a Sonar quality pass. The quick developer CLI
check is unavailable for this organization, and snippet analysis returned an
internal error. The full configured scanner, required application gates, and
disposable media integration remain acceptance requirements.
