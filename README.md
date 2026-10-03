# Revaer

Revaer is a data-driven media orchestration platform that centralizes configuration, telemetry, and operational control in PostgreSQL. The repository is organized as a Rust workspace composed of focused crates that together deliver the initial torrent + filesystem management minimal lovable product.

## Guiding Principles

-   **Single source of truth** – `DATABASE_URL` is the only required environment variable; all runtime configuration is stored in the database and hot-reloads across services.
-   **Composable crates** – Each domain area (configuration, API, engine, filesystem, telemetry) lives in its own crate with well-defined traits and DTOs.
-   **Observability first** – Telemetry, health, and structured events are wired in from the outset.
-   **Safe defaults** – Setup mode boots in a locked-down state until the operator unlocks the system through the API or CLI using a one-time token.

## Workspace Layout

```
revaer/
├─ Cargo.toml
├─ setup.sh
├─ pyproject.toml
├─ tools/                       # uv-managed rv CLI
├─ README.md
├─ docs/
│  ├─ adr/
│  └─ api/
├─ config/
│  └─ reference-config.md        # Documentation only; no runtime reads
└─ crates/
   ├─ revaer-app                 # Composition root binary crate
   ├─ revaer-api                 # Axum HTTP + SSE services
   ├─ revaer-cli                 # Terminal client
   ├─ revaer-config              # DB-backed settings service
   ├─ revaer-fsops               # Filesystem post-processing
   ├─ revaer-telemetry           # Tracing, metrics, health hooks
   ├─ revaer-torrent-core        # Engine-agnostic traits & DTOs
   └─ revaer-torrent-libt        # libtorrent adapter
```

## Getting Started

Run `./setup.sh` once to provision the uv-managed Python environment, pinned
native tools and local `rv` launcher. Then run `rv check` to verify the workspace.
See the [setup and architecture guide](tools/README.md) for installation options
and worktree behavior. The [migration inventory](tools/migration.md) records
remaining qualification and retirement work.

For automation, use `uv run --locked -- rv <command>` after the shared setup
action. Local development and CI use the same task implementations.

## Development Tasks

-   `rv fmt` – check Rust and Python formatting (`rv fmt-fix` applies formatting).
-   `rv lint` – run policy, Python static checks and Clippy with warnings as errors.
-   `rv test` – execute the full test suite (integration + unit).
-   `rv build` – build the workspace with all features enabled.
-   `rv udeps` – detect unused dependencies.
-   `rv audit` – audit Rust and locked Python dependencies for published advisories.
-   `rv deny` – enforce the license and advisory policy (`cargo-deny`).
-   `rv cov` – run source-based coverage with LLVM (requires `llvm-tools-preview`).
-   `rv sonar-compile-db` – build the native libtorrent shim and emit `coverage/compile_commands.json` for local SonarQube C-family analysis.
-   `rv ci` – execute all required quality gates locally.
-   `rv ui-e2e` – run Playwright API + UI E2E tests with temp databases and managed servers.

## UI E2E Testing

The Playwright suite lives under `tests/` and reads configuration from `tests/.env`. API checks (both auth modes) run before UI specs to catch backend failures first. Each run provisions a temp database.

1. Ensure ports `7070` and `8080` are free (the runner refuses occupied ports; stop any existing server first).
2. Update `tests/.env` as needed (notably `E2E_DB_ADMIN_URL`, `E2E_BASE_URL`, `E2E_API_BASE_URL`, `E2E_FS_ROOT`).
3. Run `rv ui-e2e`.

## Native Libtorrent Integration Test

The native libtorrent integration test suite is opt-in to keep default runs deterministic.

-   Enable it with `REVAER_NATIVE_IT=1`; it skips otherwise.
-   Ensure Docker is reachable (set `DOCKER_HOST` if not on `/var/run/docker.sock`).
-   Run `REVAER_NATIVE_IT=1 rv ci` or `rv test-native` when the native path should be covered (e.g., feature matrices).
-   See `docs/platform/native-tests.md` for the full setup and CI matrix note.

## CLI Tips

-   `revaer --output json ls` emits JSON suitable for scripting workflows (table output remains the default).
-   `revaer config get` and `revaer config set --file changes.json` provide a CLI wrapper around the `/v1/config` API so you can script updates without crafting HTTP requests manually.

## Optional OpenTelemetry Export

Set `REVAER_ENABLE_OTEL=true` to attach the OTLP tracing exporter. The exporter is disabled by default; when the flag is present the app uses `REVAER_OTEL_SERVICE_NAME` (defaults to `revaer-app`) and sends traces to `REVAER_OTEL_EXPORTER` or the standard `OTEL_EXPORTER_OTLP_ENDPOINT` when provided. This keeps the instrumentation tree dormant unless you explicitly request it in environments that provide an OTLP collector.

### Required Tooling

`rv setup` installs the exact Cargo and browser tool versions selected by
[tools/versions.toml](tools/versions.toml) and the project lockfile. Re-run it when
those pins change. Use `rv setup --help` for profile and tool selections.

## Documentation

The documentation site is powered by [mdBook](https://rust-lang.github.io/mdBook/) and mirrors the automated docs pipeline for this repository.

1. Install the tools with `rv setup` (includes pinned mdBook and Mermaid).
2. Build and index the docs with `rv docs` (runs `docs-build` followed by `docs-index`).
3. Preview locally via `rv docs-serve`.

Pushes to `main` invoke the docs workflow to rebuild the book, refresh the LLM manifests under `docs/llm/`, and publish the static site to GitHub Pages.

## Next Steps

-   Implement the migrations and configuration schema inside `revaer-config`.
-   Wire the setup flow and runtime hot-reload between `revaer-app`, `revaer-api`, `revaer-torrent-*`, and `revaer-fsops`.
-   Author ADRs capturing architectural decisions, bootstrap flow, and configuration invariants.
