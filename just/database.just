sqlx-install:
    required_sqlx_version="0.8.6"; \
    bash scripts/ensure-exact-cargo-tool.sh \
        sqlx sqlx-cli "${required_sqlx_version}" \
        --no-default-features --features postgres

db-migrate: sqlx-install
    db_url="${DATABASE_URL:-${REVAER_TEST_DATABASE_URL:-$(bash scripts/local-postgres-url.sh revaer)}}"; \
    DATABASE_URL="${db_url}" sqlx migrate run --source crates/revaer-data/migrations

# Start local Postgres only when the caller selected managed lifecycle ownership.
# Explicit reachable database URLs remain caller-owned and are never replaced by Docker.
db-start:
    database_url_was_set="0"; \
    if [ -n "${DATABASE_URL:-}" ]; then database_url_was_set="1"; fi; \
    local_db_user="${REVAER_LOCAL_DB_USER:-revaer}"; \
    local_db_password="${REVAER_LOCAL_DB_PASSWORD:-${local_db_user}}"; \
    db_url="${DATABASE_URL:-$(bash scripts/local-postgres-url.sh revaer)}"; \
    db_host="$(printf "%s" "${db_url}" | sed -E 's#^[^:]+://[^@]+@([^:/]+).*#\1#')"; \
    db_port="$(printf "%s" "${db_url}" | sed -En 's#^[^:]+://[^@]+@[^:/]+:([0-9]+).*#\1#p')"; \
    if [ -z "${db_port}" ]; then db_port="5432"; fi; \
    probe_tcp() { \
        probe_host="$1"; \
        probe_port="$2"; \
        if command -v nc >/dev/null 2>&1; then \
            nc -z -w 1 "${probe_host}" "${probe_port}" >/dev/null 2>&1; \
        elif command -v python3 >/dev/null 2>&1; then \
            python3 -c 'import socket, sys; probe = socket.create_connection((sys.argv[1], int(sys.argv[2])), 1); probe.close()' "${probe_host}" "${probe_port}" >/dev/null 2>&1; \
        else \
            echo "db-start requires nc or python3 for TCP readiness probes" >&2; \
            return 2; \
        fi; \
    }; \
    if ! command -v nc >/dev/null 2>&1 && ! command -v python3 >/dev/null 2>&1; then \
        echo "db-start requires nc or python3 for TCP readiness probes" >&2; \
        exit 1; \
    fi; \
    if [ "${db_host}" = "host.docker.internal" ] && probe_tcp localhost "${db_port}"; then \
        db_host="localhost"; \
        db_url="$(printf "%s" "${db_url}" | sed 's#@host\.docker\.internal\([:/]\)#@localhost\1#')"; \
        echo "Normalized local Docker database host to ${db_host}:${db_port}"; \
    fi; \
    if [ "${db_host}" = "localhost" ] && ! probe_tcp localhost "${db_port}" && probe_tcp host.docker.internal "${db_port}"; then \
        db_host="host.docker.internal"; \
        db_url="$(printf "%s" "${db_url}" | sed 's#@localhost\([:/]\)#@host.docker.internal\1#')"; \
        echo "Normalized local Docker database host to ${db_host}:${db_port}"; \
    fi; \
    echo "Using database endpoint: ${db_host}:${db_port}"; \
    db_lifecycle_mode="${REVAER_DB_MANAGED:-auto}"; \
    case "${db_lifecycle_mode}" in \
        auto|0|1) ;; \
        true) db_lifecycle_mode="1" ;; \
        false) db_lifecycle_mode="0" ;; \
        *) \
            echo "REVAER_DB_MANAGED must be auto, 0, 1, true, or false" >&2; \
            exit 1; \
            ;; \
    esac; \
    local_database_endpoint="0"; \
    if echo "${db_url}" | grep -Eq '@(localhost|127\.0\.0\.1|host\.docker\.internal)(:|/)'; then \
        local_database_endpoint="1"; \
    fi; \
    docker_managed_endpoint="0"; \
    if [ "${db_lifecycle_mode}" = "1" ]; then \
        if [ "${local_database_endpoint}" != "1" ]; then \
            echo "REVAER_DB_MANAGED=1 requires a localhost, 127.0.0.1, or host.docker.internal DATABASE_URL." >&2; \
            exit 1; \
        fi; \
        docker_managed_endpoint="1"; \
    elif [ "${db_lifecycle_mode}" = "auto" ] && [ "${database_url_was_set}" != "1" ] && [ "${local_database_endpoint}" = "1" ]; then \
        docker_managed_endpoint="1"; \
    fi; \
    container_name="${PG_CONTAINER:-revaer-db}"; \
    if [ "${docker_managed_endpoint}" != "1" ] && [ "${database_url_was_set}" = "1" ] && probe_tcp "${db_host}" "${db_port}"; then \
        echo "Using existing Postgres endpoint ${db_host}:${db_port}"; \
    elif [ "${docker_managed_endpoint}" = "1" ]; then \
        db_data_dir="${REVAER_DB_DATA_DIR:-${PWD}/.server_root/postgres-data}"; \
        db_shm_size="${REVAER_DB_SHM_SIZE:-1g}"; \
        required_shm_bytes="${REVAER_DB_SHM_BYTES:-1073741824}"; \
        mkdir -p "${db_data_dir}"; \
        existing_container="$(docker ps -aq -f name=^${container_name}$)"; \
        if [ -n "${existing_container}" ]; then \
            existing_shm_bytes="$(docker inspect "${container_name}" 2>/dev/null | sed -n 's/.*"ShmSize": \([0-9][0-9]*\).*/\1/p' | head -n 1)"; \
            if [ -z "${existing_shm_bytes}" ] || [ "${existing_shm_bytes}" -lt "${required_shm_bytes}" ]; then \
                echo "Recreating existing Postgres container (${container_name}) with insufficient shared memory"; \
                docker rm -f "${container_name}" >/dev/null; \
                existing_container=""; \
            fi; \
        fi; \
        if [ -n "${existing_container}" ] && [ -z "$(docker ps -q -f name=^${container_name}$)" ]; then \
            if docker logs --tail 50 "${container_name}" 2>&1 | grep -q 'No space left on device'; then \
                echo "Recreating failed Postgres container (${container_name}) with host-backed storage"; \
                if ! docker rm -f "${container_name}" >/dev/null; then \
                    echo "Failed to remove Postgres container ${container_name}." >&2; \
                    exit 1; \
                fi; \
                existing_container=""; \
            fi; \
        fi; \
        if probe_tcp "${db_host}" "${db_port}"; then \
            if [ -z "${existing_container}" ]; then \
                echo "Managed database endpoint ${db_host}:${db_port} is occupied by an unmanaged process." >&2; \
                exit 1; \
            fi; \
            echo "Using existing Postgres endpoint ${db_host}:${db_port}"; \
        else \
            if [ -n "${existing_container}" ]; then \
                if published_port="$(docker port "${container_name}" 5432/tcp 2>/dev/null)"; then :; else published_port=""; fi; \
                if [ -z "${published_port}" ]; then \
                    echo "Recreating existing Postgres container (${container_name}) without a published host port"; \
                    docker rm -f "${container_name}" >/dev/null; \
                    existing_container=""; \
                elif ! printf "%s" "${published_port}" | grep -Eq "(:|^)${db_port}$"; then \
                    echo "Recreating existing Postgres container (${container_name}) with mismatched published port ${published_port}"; \
                    docker rm -f "${container_name}" >/dev/null; \
                    existing_container=""; \
                fi; \
            fi; \
            if [ -n "${existing_container}" ]; then \
                if [ -z "$(docker ps -q -f name=^${container_name}$)" ]; then \
                    echo "Starting existing Postgres container (${container_name})"; \
                    docker start "${container_name}" >/dev/null; \
                fi; \
            else \
                echo "Starting new Postgres container (${container_name})"; \
                docker run -d \
                    --name "${container_name}" \
                    -e POSTGRES_USER="${local_db_user}" \
                    -e POSTGRES_PASSWORD="${local_db_password}" \
                    -e POSTGRES_DB=revaer \
                    --shm-size "${db_shm_size}" \
                    -p "${db_port}:5432" \
                    -v "${db_data_dir}:/var/lib/postgresql/data" \
                    postgres:16-alpine >/dev/null; \
            fi; \
            echo "Waiting for Postgres to become ready..."; \
            for _ in $(seq 1 30); do \
                if docker exec "${container_name}" pg_isready -U "${local_db_user}" -d postgres >/dev/null 2>&1; then break; fi; \
                sleep 1; \
            done; \
        fi; \
    elif probe_tcp "${db_host}" "${db_port}"; then \
        echo "Using existing Postgres endpoint ${db_host}:${db_port}"; \
    else \
        echo "Postgres endpoint ${db_host}:${db_port} is unavailable and managed mode was not requested." >&2; \
        exit 1; \
    fi; \
    external_ready="0"; \
    for _ in $(seq 1 30); do \
        if probe_tcp "${db_host}" "${db_port}"; then external_ready="1"; break; fi; \
        sleep 1; \
    done; \
    if [ "${external_ready}" != "1" ] && [ "${db_host}" = "localhost" ] && probe_tcp host.docker.internal "${db_port}"; then \
        db_host="host.docker.internal"; \
        db_url="$(printf "%s" "${db_url}" | sed 's#@localhost\([:/]\)#@host.docker.internal\1#')"; \
        external_ready="1"; \
    fi; \
    if [ "${external_ready}" != "1" ]; then \
        echo "Postgres endpoint ${db_host}:${db_port} did not become reachable." >&2; \
        exit 1; \
    fi; \
    wait_for_local_postgres_writable() { \
        attempt="1"; \
        while [ "${attempt}" -le 60 ]; do \
            writable="$(docker exec -e PGPASSWORD="${local_db_password}" "${container_name}" psql -U "${local_db_user}" -d postgres -Atqc 'SELECT CASE WHEN pg_is_in_recovery() THEN 0 ELSE 1 END' 2>/dev/null | tr -d '[:space:]')"; \
            if [ "${writable}" = "1" ]; then return 0; fi; \
            attempt="$((attempt + 1))"; \
            sleep 1; \
        done; \
        echo "Local Postgres container ${container_name} did not exit recovery in time." >&2; \
        return 1; \
    }; \
    if [ "${docker_managed_endpoint}" = "1" ]; then wait_for_local_postgres_writable; fi; \
    just sqlx-install; \
    run_sqlx_with_recovery_retry() { \
        attempt="1"; \
        while true; do \
            output="$("$@" 2>&1)"; \
            status=$?; \
            if [ "${status}" -eq 0 ]; then \
                if [ -n "${output}" ]; then printf '%s\n' "${output}"; fi; \
                return 0; \
            fi; \
            if printf '%s' "${output}" | grep -Eq 'the database system is (in recovery mode|starting up|not yet accepting connections)|consistent recovery state has not been yet reached'; then \
                printf '%s\n' "${output}" >&2; \
                if [ "${attempt}" -ge 180 ]; then return 2; fi; \
                attempt="$((attempt + 1))"; \
                sleep 1; \
                continue; \
            fi; \
            printf '%s\n' "${output}" >&2; \
            return "${status}"; \
        done; \
    }; \
    if create_output="$(DATABASE_URL="${db_url}" sqlx database create --database-url "${db_url}" 2>&1)"; then \
        if [ -n "${create_output}" ]; then printf '%s\n' "${create_output}"; fi; \
    elif ! printf '%s' "${create_output}" | grep -Eqi 'already exists'; then \
        printf '%s\n' "${create_output}" >&2; \
        exit 1; \
    fi; \
    reset_db="${REVAER_DB_RESET:-0}"; \
    if [ "${reset_db}" = "1" ]; then \
        if [ "${docker_managed_endpoint}" != "1" ]; then \
            echo "Reset requested for ${db_host}:${db_port}; refusing to reset a caller-owned database." >&2; \
            exit 1; \
        fi; \
        run_sqlx_with_recovery_retry env DATABASE_URL="${db_url}" sqlx database reset -y --database-url "${db_url}" --source crates/revaer-data/migrations; \
    else \
        if run_sqlx_with_recovery_retry env DATABASE_URL="${db_url}" sqlx migrate run --database-url "${db_url}" --source crates/revaer-data/migrations; then \
            migrate_status="0"; \
        else \
            migrate_status="$?"; \
        fi; \
        if [ "${migrate_status}" -eq 0 ]; then exit 0; fi; \
        if [ "${migrate_status}" -eq 2 ]; then exit 1; fi; \
        if [ "${docker_managed_endpoint}" != "1" ]; then \
            echo "Migration history mismatch for ${db_host}:${db_port}; refusing to reset a caller-owned database." >&2; \
            exit "${migrate_status}"; \
        fi; \
        echo "Migration history mismatch; resetting managed local database..."; \
        run_sqlx_with_recovery_retry env DATABASE_URL="${db_url}" sqlx database reset -y --database-url "${db_url}" --source crates/revaer-data/migrations; \
    fi

db-reset:
    REVAER_DB_MANAGED=1 REVAER_DB_RESET=1 just db-start

db-seed:
    local_db_user="${REVAER_LOCAL_DB_USER:-revaer}"; \
    db_url="${DATABASE_URL:-$(bash scripts/local-postgres-url.sh revaer)}"; \
    db_managed="${REVAER_DB_MANAGED:-0}"; \
    if [ -z "${DATABASE_URL:-}" ]; then db_managed="1"; fi; \
    REVAER_DB_MANAGED="${db_managed}" DATABASE_URL="${db_url}" just db-start; \
    cat scripts/dev-seed.sql | docker exec -i "${PG_CONTAINER:-revaer-db}" psql -U "${local_db_user}" -d revaer >/dev/null
