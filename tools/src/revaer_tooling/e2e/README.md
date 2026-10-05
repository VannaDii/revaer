# API and browser support

## Migration status

The HTTP client, schema validation, settings, route recorder, Cargo artifact
selection, and service ownership checks are implemented and tested. The foundation
passes `rv ui-e2e` and `rv ui-e2e-coverage` in its supported Linux environment:
35 anonymous API scenarios, 35 authenticated API scenarios, and 13 Chromium
scenarios. Real browser fixture tests also cover Firefox and WebKit. Full media
scenario integration and the remaining project CI migration are still pending;
see the [inventory](../../../migration.md).

## Architecture

`UiE2e.run(context)` owns the run. It acquires a checkout-local lock, invalidates
old results, builds the selected API executable through Cargo, creates a unique
database, and starts the API and Trunk as owned process groups. It verifies the
actual listener PID before calling setup or factory reset. Cleanup stops and
reaps the services before dropping the unique database.

For `feature-development` checkouts, database setup uses the single initializer
through `rv db-test-init`/`db-test-drop`. Set `E2E_DB_ADMIN_URL` (or the existing
test URL fallback), `PG_CONTAINER`, and the test-service admin credentials. The
URL must match the selected container's exact loopback port with no URI overrides.
The coordinator generates a fresh temporary password and hands only the sealed
runtime role to the application. It neither starts nor adopts a service in this
mode. Invalid transition configuration fails instead of reverting to migrations.
Other supported phases retain their existing foundation migration path.

The phases are explicit and ordered:

1. `api-none` configures anonymous access and runs the API scenarios.
2. `api-api-key` configures key authentication and runs the same scenarios.
3. `ui-<browser>` runs the browser scenarios using the resulting key.

Single-init media expands each API authentication mode into an absent-catalog
phase followed by its positive API phase: `api-none-missing-catalog`, `api-none`,
`api-api-key-missing-catalog`, `api-api-key`, then the browser phases. The original
three missing-catalog checks run in each dedicated phase; the positive phases
retain every foundation scenario and all other media scenarios. Each phase has
its own selection, report and raw coverage, required by shard and coverage
consumers. This separation preserves rejection checks alongside real discovery.

Authentication setup's factory reset clears catalog tables. The media runner
performs that setup with an absent startup source, stops and reaps that service,
then starts the scenario service. Positive phases require the real configured
`REVAER_MEDIA_ROOT_CATALOG_FILE` to attest successfully on Linux. No stored
attestation is inserted by the harness. Each API phase reset uses the current
session retained from the preceding phase, then replaces it with the new setup
credentials. Restarting the service does not reset database authentication.
Browser restarts retain the authenticated
session and database without another reset. Setup and scenario logs live in
separate phase subdirectories under `tests/logs`.

API phases use one worker because they share mutable application configuration.
UI workers are configurable. Keys pass to pytest through its environment, with
redaction at the process boundary. Process logs are private files and reject
symlinks or extra hard links. No persistent plaintext session file is published.

`ApiRequest` contains a method, route template, typed path/query values, optional
JSON body, and headers. `ApiClient` injects the HTTP transport, selected origin,
OpenAPI schema, and route recorder. JSON accessors check object/array/string
shapes. Response validation resolves schemas from the checkout's actual
`docs/api/openapi.json`; mismatch messages omit response values that might hold
new credentials. Streaming APIs use bounded reads and always close their stream.

Route evidence measures observed API operations and UI navigation. It is separate
from Python line/branch coverage and from the assertions in each test. A route
being observed does not imply its test passed. `rv ui-e2e-coverage` requires every
OpenAPI operation and every UI route configured in `tests/e2e.toml`.

`tests/conftest.py` is pytest's wiring layer. Each test gets a native Playwright
context seeded with storage state. Each retry owns a separate artifact directory.
Page events retain references to closed pages so their videos are not lost.
Cleanup attempts JavaScript capture, screenshots, tracing, context closure and
video handling in that order. A failed video save preserves its raw recording; successful cleanup removes
scratch recordings, including browser-internal storage-restoration pages.
Local HTTP 500 responses fail the browser scenario even if a placeholder or
loading indicator rendered successfully. Diagnostics omit query strings, headers,
and bodies. Setup failures and timeouts retain their artifacts and close contexts.

Route recorders live for the pytest session and write separate files per worker.
Collection sorts scenario IDs and partitions complete scenarios across shards.
Selection manifests record both the full collection and the selected IDs. An
expected empty shard succeeds; a genuinely empty suite remains a failure.
This distinction also holds with xdist workers, whose collection counts are
transferred to the controller through the plugin's worker-completion hook.

### Combining CI shards

Download the results from the current workflow attempt into one clean directory:

```console
E2E_COVERAGE_DIR=artifacts/e2e-results rv ui-e2e-shard-coverage --shards 3
E2E_COVERAGE_DIR=artifacts/e2e-results rv ui-e2e-coverage
```

The first command verifies completion summaries, agreement between workers,
deterministic scenario assignments across every expected phase/shard, and nonempty
route records for every nonempty selection. Setup requests cannot replace API
scenario evidence. The second checks the combined route requirements. The shard
count defaults to three, matching existing media CI, and is explicitly overridable.
Do not mix artifacts from different workflow attempts.

## Configuration

`rv ui-e2e` loads `tests/.env` using the official
[`uv run --env-file` interface](https://docs.astral.sh/uv/concepts/projects/run/#environment-files).
Existing environment variables keep uv's documented precedence. The tool does
not implement a dotenv parser. The internal re-entry flag only prevents a second
load after uv has prepared the environment.

| Setting | Default and behavior |
| --- | --- |
| `E2E_API_BASE_URL` | `http://localhost:7070`; the owned API bootstraps on port 7070. |
| `E2E_BASE_URL` | `http://localhost:8080`; selects the local Trunk origin and port. |
| `E2E_DB_ADMIN_URL` | Falls back to `REVAER_TEST_DATABASE_URL`, then `DATABASE_URL`, then the managed local server. Only a newly created database is dropped. |
| `E2E_SKIP_DB_START` | False; true uses an available server without starting Docker. |
| `E2E_DB_PREFIX` | `revaer_e2e`; a lowercase identifier up to 30 characters, followed by a random run identifier. |
| `E2E_FS_ROOT` | Checkout root; relative overrides resolve against the selected checkout. Setup allowlists this fixture root. |
| `E2E_API_RUNNER` | Overrides `[api].runner` in `tests/e2e.toml`: `binary` or media's `lib-test`. |
| `E2E_BROWSERS` | `chromium`; comma-separated `chromium`, `firefox`, `webkit`. |
| `E2E_BROWSER_CHANNEL` | Optional installed browser channel passed to Playwright. |
| `E2E_BROWSER_COVERAGE` | False; true records Chromium JavaScript execution and requires `ui-chromium` among the selected phases. |
| `E2E_HEADLESS` | True. |
| `E2E_VIEWPORT_WIDTH`, `E2E_VIEWPORT_HEIGHT` | 1440 × 900. |
| `E2E_RETRIES` | Two in CI, zero locally. |
| `E2E_UI_WORKERS`, `E2E_WORKERS` | UI defaults to one; the global worker limit caps it when supplied. |
| `E2E_TEST_TIMEOUT_MS` | 30000; pytest-timeout's signal mode preserves teardown on supported Unix hosts. |
| `E2E_EXPECT_TIMEOUT_MS` | 5000. |
| `E2E_ACTION_TIMEOUT_MS`, `E2E_NAVIGATION_TIMEOUT_MS` | 10000 and 15000. |
| `E2E_TRACE`, `E2E_VIDEO` | `on-first-retry` and `retain-on-failure`; `on` and `off` are also supported modes. |
| `E2E_SCREENSHOT` | `only-on-failure`; also `on` or `off`. |
| `E2E_HTTP_WAIT_SECONDS`, `E2E_HTTP_WAIT_INTERVAL_MS` | 120 seconds and 500 ms; `E2E_HTTP_WAIT_ATTEMPTS` takes precedence over the seconds setting. |
| `E2E_PLAYWRIGHT_PROJECTS` | All configured phases; selected phases include their dependencies unless `E2E_PLAYWRIGHT_NO_DEPS` is true. |
| `PLAYWRIGHT_SHARD_INDEX`, `PLAYWRIGHT_SHARD_TOTAL` | One-based shard identity; index must not exceed total. |
| `E2E_COVERAGE_DIR` | Overrides the directory read by the functional route-coverage gate. |

Invalid settings fail before service startup. Dependency-free focused runs still
configure authentication against their disposable database; they do not adopt a
session from an unrelated running application.

## Evidence and failures

- `tests/logs/` contains private service logs.
- `tests/playwright-report/index.html` links the phase reports and current status.
- `tests/test-results/` contains phase artifacts, JUnit XML, route coverage, and
  `python-e2e-summary.json` (or `python-e2e-summary-shard-N.json`).
- `coverage/e2e/<phase>/` contains that phase's measured Python XML, LCOV, and raw
  coverage data. A phase is partial evidence, so it does not apply the independent
  tooling runtime's 90% aggregate gate to an incomplete run.

Build/startup failures retain a failed current-run summary. A failed phase stops
the dependent phases, preserves its evidence, and makes the command fail. An
occupied port is reported to its operator; the runner never kills by port/name.

## JavaScript source coverage

The Sonar run enables `E2E_BROWSER_COVERAGE=1`, completes the unsharded API and
browser phases, then calls `rv js-coverage-merge` with the same settings. This
publishes `coverage/js-lcov.info`. Functional operation/route evidence remains
independent of this line coverage.

There are two inputs, each with a separate responsibility:

1. Tree-sitter's official Python bindings and JavaScript grammar parse every
   tracked or nonignored working `.js`, `.mjs`, and `.cjs` file. The inventory
   includes unexecuted modules without fetching imports or running their code.
   Named code tokens, statements and declarations supply source locations;
   comments and punctuation-only containers do not. Every location starts at zero.
2. Chromium's native precise-coverage API supplies the execution counts. Source
   SHA-256 and UTF-16 length from its script events bind those counters to the
   exact parsed bytes. The narrowest containing V8 range determines each count,
   so an unexecuted nested branch stays at zero inside an executed function.

The merger combines the measured locations into line records. A line is covered
when at least one of its code locations ran. Identical source copies share the
same measurement but retain distinct checkout paths. Generated Trunk loaders
without matching authored bytes keep their raw captures; their execution is not
attributed to an unrelated source file. This reports line coverage, not an
independent statement or branch percentage.

The `page` fixture is instrumented before a scenario receives it. Scenarios that
need another page request `new_page: Callable[[], Page]`; scenarios that close a
page early request `close_page: Callable[[Page], None]`. These helpers also work
with coverage disabled and with Firefox/WebKit. The close helper captures pending
execution before closing the native page. Creating pages directly on the context
or through an automatic popup currently fails a coverage-enabled run; add an
explicitly verified lifecycle when a scenario requires such a popup.

Partial captures, syntax errors, changed sources, missing files, invalid counters,
worker disagreement and unsuccessful/partial runs fail the merge. It holds the
E2E operation lock while reading and publishing. A capture failure retains traces
and videos alongside the private partial JSON. Baselines are
`tests/test-results/javascript-baseline-*.json`; per-attempt captures are
`tests/test-results/ui-chromium/*/javascript-page-*.json`. A new run invalidates
the old inputs before collection. Never combine evidence across workflow attempts.

## Upstream interfaces

- [Playwright pytest integration](https://playwright.dev/python/docs/test-runners)
- [Playwright Python API testing](https://playwright.dev/python/docs/api-testing)
- [Playwright CDP sessions](https://playwright.dev/python/docs/api/class-cdpsession)
- [V8 precise coverage protocol](https://chromedevtools.github.io/devtools-protocol/tot/Profiler/)
- [Tree-sitter Python bindings](https://github.com/tree-sitter/py-tree-sitter) and
  [JavaScript grammar](https://github.com/tree-sitter/tree-sitter-javascript)
- [pytest-timeout 2.4.0](https://pypi.org/project/pytest-timeout/2.4.0/); 2.5.0 was
  [withdrawn by its maintainers](https://pypi.org/project/pytest-timeout/2.5.0/).
- [Cargo JSON artifact messages](https://doc.rust-lang.org/cargo/reference/external-tools.html#json-messages)

## Media scenario selection

The `feature-development` phase adds `tests/specs/media/api` to both API
authentication phases; it retains `tests/specs/api`. Other phases run the
foundation cases. The first media ports cover root-catalog readiness, profile
creation/update guards and manual/disabled-automation discovery (14 cases).
Remaining media API and UI scenarios are still tracked as migration work.

`tests/e2e.toml` specifies `runner` for the foundation and `media_runner` for
single-init media. The latter uses the original Rust library test-serving entry.
`E2E_API_RUNNER` can override either default. A production binary still requires
its packaged compliance metadata; selecting the media test entry does not change
production validation or qualify a release image.
