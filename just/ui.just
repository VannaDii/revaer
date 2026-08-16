sync-assets:
    cargo run -p asset_sync

check-assets: sync-assets
    git diff --exit-code -- crates/revaer-ui/static/nexus

trunk-install:
    required_trunk_version="0.21.14"; \
    bash scripts/ensure-exact-cargo-tool.sh \
        trunk trunk "${required_trunk_version}"

ui-serve: sync-assets trunk-install
    rustup target add wasm32-unknown-unknown
    mkdir -p crates/revaer-ui/dist-serve/.stage
    cd crates/revaer-ui && NO_COLOR=true trunk serve --dist dist-serve --open

ui-build: sync-assets trunk-install
    rustup target add wasm32-unknown-unknown
    mkdir -p crates/revaer-ui/dist/.stage
    cd crates/revaer-ui && NO_COLOR=true trunk build --release

api-test-client:
    bash scripts/with-node.sh npm --prefix tests ci --ignore-scripts
    bash scripts/with-node.sh npm --prefix tests audit --audit-level=info
    bash scripts/with-node.sh npm --prefix tests run gen:api-client
    test -s tests/support/api/schema.ts

ui-e2e: trunk-install api-test-client
    if [ "${CI:-}" = "true" ] || { [ "$(uname -s)" = "Linux" ] && sudo -n true >/dev/null 2>&1; }; then \
        cd tests && bash ../scripts/with-node.sh npx playwright install --with-deps; \
    else \
        cd tests && bash ../scripts/with-node.sh npx playwright install; \
    fi
    bash scripts/with-node.sh tests/node_modules/.bin/tsc --project tests/tsconfig.coverage.json
    shard_arg=""; \
    coverage_suffix=""; \
    if [ -n "${PLAYWRIGHT_SHARD_INDEX:-}" ] && [ -n "${PLAYWRIGHT_SHARD_TOTAL:-}" ]; then \
        shard_arg="--shard=${PLAYWRIGHT_SHARD_INDEX}/${PLAYWRIGHT_SHARD_TOTAL}"; \
        coverage_suffix="-shard-${PLAYWRIGHT_SHARD_INDEX}"; \
    fi; \
    coverage_dir="coverage/js/playwright${coverage_suffix}"; \
    state_key="$(bash scripts/with-node.sh node -e "process.stdout.write(require('node:crypto').randomBytes(32).toString('hex'))")"; \
    rm -rf "${coverage_dir}"; \
    REVAER_E2E_STATE_KEY="${state_key}" \
    E2E_ENV_DIR="${PWD}/tests" \
    NODE_PATH="${PWD}/tests/node_modules" env -u NO_COLOR bash scripts/with-node.sh tests/node_modules/.bin/c8 \
        --reporter=lcovonly \
        --reports-dir "${coverage_dir}" \
        --include 'target/js-coverage-tests/**/*.js' \
        --exclude-after-remap=false \
        bash scripts/with-node.sh tests/node_modules/.bin/playwright test \
        --config target/js-coverage-tests/playwright.config.js \
        ${shard_arg}; \
    test -s "${coverage_dir}/lcov.info"; \
    grep -q '^SF:tests/' "${coverage_dir}/lcov.info"; \
    test "$(grep -c '^DA:' "${coverage_dir}/lcov.info")" -ge 1000; \
    grep -Eq '^DA:[0-9]+,[1-9][0-9]*(,|$)' "${coverage_dir}/lcov.info"; \
    grep -Eq '^DA:[0-9]+,0(,|$)' "${coverage_dir}/lcov.info"

ui-e2e-coverage:
    bash scripts/with-node.sh node tests/scripts/check-e2e-coverage.js

ui-e2e-shard-coverage:
    ruby scripts/verify-ui-e2e-shard-coverage.rb

runbook:
    status=0; \
    just ui-e2e || status=$?; \
    if [ "$status" -eq 0 ]; then \
        just ui-e2e-coverage || status=$?; \
    fi; \
    mkdir -p artifacts/runbook; \
    rm -rf artifacts/runbook/logs artifacts/runbook/playwright-report artifacts/runbook/test-results; \
    if [ -d tests/logs ]; then \
        cp -R tests/logs artifacts/runbook/logs; \
    fi; \
    if [ -d tests/playwright-report ]; then \
        cp -R tests/playwright-report artifacts/runbook/playwright-report; \
    fi; \
    if [ -d tests/test-results ]; then \
        cp -R tests/test-results artifacts/runbook/test-results; \
        rm -f artifacts/runbook/test-results/e2e-state.json; \
    fi; \
    runbook_status="ok"; \
    if [ "$status" -ne 0 ]; then \
        runbook_status="failed"; \
    fi; \
    printf '%s\n' \
        "runbook=${runbook_status}" \
        "artifacts=artifacts/runbook" \
        "playwright_report=artifacts/runbook/playwright-report/index.html" \
        "test_results=artifacts/runbook/test-results" \
        "logs=artifacts/runbook/logs" \
        > artifacts/runbook/summary.txt; \
    exit "$status"

zombies:
    for port in 7070 8080; do \
        pids=$(lsof -ti :$port 2>/dev/null || true); \
        if [ -z "$pids" ]; then \
            continue; \
        fi; \
        echo "Stopping processes on port $port: $pids"; \
        kill $pids 2>/dev/null || true; \
        for pid in $pids; do \
            for _ in 1 2 3 4 5 6 7 8 9 10; do \
                if ! kill -0 "$pid" 2>/dev/null; then \
                    break; \
                fi; \
                sleep 0.2; \
            done; \
            if kill -0 "$pid" 2>/dev/null; then \
                echo "Force killing process $pid on port $port"; \
                kill -9 "$pid" 2>/dev/null || true; \
            fi; \
        done; \
        remaining=$(lsof -ti :$port 2>/dev/null || true); \
        if [ -n "$remaining" ]; then \
            echo "Processes still bound to port $port: $remaining" >&2; \
            exit 1; \
        fi; \
    done

dev: sync-assets trunk-install
    db_url="${DATABASE_URL:-$(bash scripts/local-postgres-url.sh revaer)}"; \
    db_managed="${REVAER_DB_MANAGED:-0}"; \
    if [ -z "${DATABASE_URL:-}" ]; then db_managed="1"; fi; \
    REVAER_DB_MANAGED="${db_managed}" DATABASE_URL="${db_url}" just db-start; \
    check_port_free() { \
        port="$1"; \
        name="$2"; \
        if [ "${DEV_SKIP_PORT_CHECK:-0}" = "1" ]; then \
            return 0; \
        fi; \
        if (echo >/dev/tcp/127.0.0.1/"${port}") >/dev/null 2>&1; then \
            echo "${name} port ${port} is already in use. Stop the existing service or set DEV_SKIP_PORT_CHECK=1 to skip." >&2; \
            return 1; \
        fi; \
    }; \
    check_port_free 7070 "API"; \
    check_port_free 8080 "UI"; \
    if ! command -v cargo-watch >/dev/null 2>&1; then \
        cargo install cargo-watch; \
    fi; \
    rustup target add wasm32-unknown-unknown; \
    DATABASE_URL="${db_url}" RUST_LOG=${RUST_LOG:-debug} cargo watch \
        --ignore 'docs/api/openapi.json' \
        --ignore 'crates/revaer-ui/dist/**' \
        --ignore 'crates/revaer-ui/dist-serve/**' \
        --ignore 'artifacts/**' \
        -x "run -p revaer-app" & \
    api_pid=$!; \
    mkdir -p crates/revaer-ui/dist-serve/.stage; \
    ( cd crates/revaer-ui && DATABASE_URL="${db_url}" RUST_LOG=${RUST_LOG:-info} NO_COLOR=true trunk serve --dist dist-serve ) & \
    ui_pid=$!; \
    terminate_pid() { \
        pid="$1"; \
        if [ -z "$pid" ]; then \
            return 0; \
        fi; \
        if ! kill -0 "$pid" 2>/dev/null; then \
            return 0; \
        fi; \
        kill "$pid" 2>/dev/null || true; \
        for _ in $(seq 1 25); do \
            if ! kill -0 "$pid" 2>/dev/null; then \
                return 0; \
            fi; \
            sleep 0.2; \
        done; \
        kill -9 "$pid" 2>/dev/null || true; \
    }; \
    kill_tree() { \
        pid="$1"; \
        if [ -z "$pid" ]; then \
            return 0; \
        fi; \
        children="$(ps -o pid= --ppid "$pid" 2>/dev/null || true)"; \
        for child in $children; do \
            kill_tree "$child"; \
        done; \
        terminate_pid "$pid"; \
    }; \
    cleanup_dev() { \
        trap - EXIT INT TERM; \
        kill_tree "$api_pid"; \
        kill_tree "$ui_pid"; \
    }; \
    trap cleanup_dev EXIT INT TERM; \
    wait_for_port() { \
        port="$1"; \
        name="$2"; \
        pid="$3"; \
        timeout="${DEV_STARTUP_TIMEOUT:-180}"; \
        deadline=$((SECONDS + timeout)); \
        while [ "$SECONDS" -lt "$deadline" ]; do \
            if ! kill -0 "$pid" 2>/dev/null; then \
                echo "${name} exited before it opened port ${port}" >&2; \
                return 1; \
            fi; \
            if (echo >/dev/tcp/127.0.0.1/"${port}") >/dev/null 2>&1; then \
                return 0; \
            fi; \
            sleep 0.2; \
        done; \
        echo "Timed out waiting for ${name} to listen on port ${port}" >&2; \
        return 1; \
    }; \
    echo "Waiting for API (7070) and UI (8080) to start..."; \
    wait_for_port 7070 "API" "$api_pid"; \
    wait_for_port 8080 "Trunk" "$ui_pid"; \
    wait $api_pid $ui_pid
