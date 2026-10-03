# Revaer API and browser tests

The Python suite runs 35 API scenarios in each authentication mode, followed by
13 browser scenarios. The foundation passes application E2E and route coverage
in its supported Linux environment. Media integration and the remaining project CI
migration are tracked in the [migration inventory](../tools/migration.md).

## Run

After `./setup.sh`, use the installed worktree-aware CLI:

```console
rv ui-e2e
rv ui-e2e-coverage
```

The runner builds Revaer, creates and migrates a unique database, starts its own
API and Trunk processes, runs ordered phases, and cleans up. Ports 7070 and 8080
must be free. It never terminates another process merely because it uses a port.

Set `E2E_DB_ADMIN_URL` to use an existing test server. Otherwise the runner uses
the configured disposable test endpoint or provisions the managed local server.
Only the new database is dropped. uv loads `tests/.env`; existing environment
variables retain uv's normal precedence. The [browser support guide](../tools/src/revaer_tooling/e2e/README.md)
documents every setting, lifecycle rule and failure behavior.

```console
E2E_BROWSERS=chromium,firefox,webkit rv ui-e2e
E2E_UI_WORKERS=2 rv ui-e2e
uv run --locked pytest tests/specs --collect-only
```

Collection is read-only and requires no application services. API phases always
use one worker because they share mutable application configuration.

## Layout

| Location | Responsibility |
| --- | --- |
| `conftest.py` | Inject API sessions, schema and route recorders; own each attempt's browser context and artifacts. |
| `e2e.toml` | Select the API serving target and required UI routes for this checkout. |
| `specs/api/` | Anonymous and authenticated API scenarios, including stateful lifecycles. |
| `specs/ui/` | Navigation, controls, layout and log filtering. |
| `pages/` | Native Playwright locators and reusable page assertions. |
| `support/` | Typed scenario setup, response assertions and bounded polling. |
| `../tools/src/revaer_tooling/e2e/` | Typed requests, OpenAPI validation, settings and route/shard evidence. |

## Writing scenarios

Keep dependent CRUD steps within one scenario so retries and shards cannot split
setup from assertions. Setup must produce real IDs; never substitute random IDs
or silently skip assertions after a failed creation. Requests validate responses
against the checkout's [OpenAPI contract](../docs/api/readme.md).

Use native Playwright storage state, locators and trial clicks. Do not patch
browser globals with JavaScript. A local HTTP 500 response fails the browser test,
including when the page renders a loading indicator or placeholder. Error
diagnostics omit credentials and response bodies.

## Reports and CI shards

Open `playwright-report/index.html` for phase results. `test-results/` contains
JUnit results, per-attempt recordings, route/selection manifests, and completion
summaries. `logs/` retains private service logs. Failed setup, timeouts, interrupted
runs and failed assertions retain evidence. Successful cleanup removes recordings
that the selected retention mode does not require.

CI downloads only the current attempt's shard artifacts into a clean directory:

```console
E2E_COVERAGE_DIR=artifacts/e2e-results rv ui-e2e-shard-coverage --shards 3
E2E_COVERAGE_DIR=artifacts/e2e-results rv ui-e2e-coverage
```

The shard check verifies completion, consistent collections and exact scenario
assignments. The route check verifies every documented API operation and configured
UI route. Neither collected tests nor observed routes alone prove a passing run.

## Migration baseline

[The scenario map](MIGRATION.md) relates the retained TypeScript files to their
Python replacements. Just, TypeScript and the existing workflows remain available
while the rest of the project migration is verified. The legacy runner is still
`just ui-e2e`; it is not the implementation used by `rv ui-e2e`.

### Media browser scenarios

The single-init feature phase also selects `specs/media/ui`. Root-readiness tests
intercept only their readiness endpoint to exercise unavailable/empty/denied
states and desktop/mobile presentation. They retain screenshots in each test's
attempt artifact directory. These transport-controlled checks do not establish
filesystem attestation or destructive-operation readiness. Foundation browser
scenarios remain selected alongside media scenarios.
