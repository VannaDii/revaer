# Local development

`rv dev` runs Revaer's API and Trunk development server. Python coordinates the
processes; Cargo builds and runs the application, and Trunk keeps its existing
Wasm build and live-reload behavior.

## Commands

```console
rv dev
rv zombies
```

Run `rv zombies` from a second terminal in the **same checkout** to stop a
development session or recover its recorded processes after an interrupted
supervisor. Ctrl+C in the development terminal performs the same normal child
cleanup. The database remains available after either operation.

Run `./setup.sh` first. The locked `dev` dependency group includes `watchfiles`
and `psutil`; the setup command installs the pinned native tools. `rv dev`
verifies Trunk's version and ensures the Wasm target is installed. It does not
install a second watcher through Cargo.

## Startup and rebuilds

1. Acquire the checkout's development lock and check for a recovery receipt.
2. Check API port **7070** and UI port **8080** before changing the database.
3. Verify Trunk, ensure the Wasm target and synchronize generated UI assets.
4. Prepare the selected database using the shared ownership and migration code.
5. Start `cargo run --locked --package revaer-app` and `trunk serve --dist dist-serve`.
6. Check that each listener belongs to its recorded process session.

`watchfiles` supplies native file notifications and event batching. Git applies
the checkout's ignore rules, including nested rules and tracked-file behavior.
The original generated-output exceptions remain: `docs/api/openapi.json`, UI
`dist` and `dist-serve`, and `artifacts`. Cargo's `target` output is also excluded.
The watcher's normal editor/cache exclusions remain in effect.

A source edit stops the previous API process group before starting the next
Cargo run. Trunk continues watching and rebuilding the UI. An unsuccessful Cargo
build or an exited API prints its result and waits for another edit; it does not
restart repeatedly against unchanged code. A Trunk exit, watcher failure,
ownership failure or startup timeout stops the session and returns an error.

## Configuration

Settings are read once at the CLI boundary and passed as immutable values.

| Setting | Default | Effect |
| --- | --- | --- |
| `DATABASE_URL` | Shared local PostgreSQL defaults | Select the application database. An explicitly supplied URL is caller-owned unless managed mode is selected. |
| `REVAER_DB_MANAGED` | `auto` | Use the existing database ownership and migration rules. |
| `DEV_STARTUP_TIMEOUT` | `180` seconds | Bound initial service startup and each API restart. A failed build waits for an edit. |
| `DEV_SKIP_PORT_CHECK` | `0` | `1` skips the initial occupied-port check. Readiness still requires ownership. |
| `RUST_LOG` | API: `debug`; Trunk: `info` | Override both native processes' log filters. |

The managed database lock spans the development session. This prevents another
`rv` operation from resetting the same managed server while the application is
running. Caller-owned databases are never reset implicitly. See the
[database lifecycle guide](../tasks/README.md#database-lifecycle) for URL
normalization, container ownership and reset controls.

`watchfiles` retains its documented
[polling controls](https://watchfiles.helpmanual.io/api/watch/#force-polling),
including `WATCHFILES_FORCE_POLLING` and `WATCHFILES_POLL_DELAY_MS`, for filesystems
that do not supply reliable notifications. Permission failures remain errors.

## Process ownership and recovery

State lives in `target/rv-dev/`:

| File | Purpose |
| --- | --- |
| `run.lock` | Only one development supervisor per checkout. |
| `session.json` | Private atomic record of the checkout, supervisor and native worker identities. Removed only after cleanup succeeds. |
| `api-1.log`, `api-2.log`, … | One private log per API attempt during a session. Output also appears in the terminal. |
| `trunk-serve.log` | Private Trunk output for the session, also streamed to the terminal. |

Each native worker starts in a separate Unix session. Its identity includes its
PID, creation time and an independently generated launch token inherited by its
children. `psutil` checks process creation identity before signalling. Recovery
also checks the user, the original worker's checkout and session membership.
The token distinguishes orphaned children from a later session that reused the
same numeric ID. Receipts contain no database URL, credentials or command output.

Cleanup preflights recorded ownership, requests normal supervisor termination,
then stops any surviving recorded worker sessions with bounded TERM/KILL waits.
An inspection or ownership failure retains the receipt and returns an error.
Unlinked state directories and regular receipt files are required. A new `rv dev`
session refuses to overwrite an unresolved receipt.

An unrelated service on 7070 or 8080 must be stopped by its owner. `rv zombies`
does not adopt it from its port, executable name or current working directory.
Processes started outside `rv dev` have no recovery receipt; the command reports
that fact. Separate `rv ui-serve` and `rv docs-serve` invocations retain their own
foreground cancellation behavior.

## Implementation

- [`settings.py`](settings.py): validated environment inputs.
- [`watching.py`](watching.py): the `watchfiles` API and native Git filtering.
- [`processes.py`](processes.py): typed identities, process inspection and recovery.
- [`session.py`](session.py): bounded receipt schema and atomic file publication.
- [`loop.py`](loop.py): API restarts, readiness and coordinated cleanup.
- [`tasks/development.py`](../tasks/development.py): the static CLI task entry points.

The CLI constructs the watcher, process manager and monotonic clock. The loop
receives those collaborators through `Context`; it does not read the environment
or construct alternative tool implementations. Optional SDK imports are lazy so
container commands can run with only the core dependency group.

## Validation

The focused tests use real `watchfiles` notifications, native Git ignore rules
and real Unix processes. They exercise creation/modification/deletion, ignored
output, PID reuse, orphan recovery, checkout isolation and TERM/KILL escalation.
Coordinator tests inject failed builds, Trunk exits, timeouts, watcher failures
and cleanup errors. Separate database tests exercise the shared native lifecycle.
Full application development and combined-worktree acceptance are additional
requirements recorded in the [migration inventory](../../../migration.md).

The dependencies replace existing responsibilities: `watchfiles` replaces the
watcher, while `psutil` replaces platform-specific process-table parsing and
signal/wait loops. The `types-psutil` package provides strict static checking.
All are pinned through uv's supported dependency resolver.
