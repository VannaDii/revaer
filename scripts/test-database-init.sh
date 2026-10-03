#!/usr/bin/env bash
set -euo pipefail

operation="${1:-}"
name="${2:-}"
if [[ $# -ne 2 || ! "${name}" =~ ^revaer_test_[0-9]+_[0-9]+$ || ${#name} -gt 50 ]]; then
    echo "Expected init/drop and a unique owned test database name." >&2
    exit 1
fi
if [[ "${operation}" != init && "${operation}" != drop ]]; then
    echo "Expected init or drop." >&2
    exit 1
fi
: "${PG_CONTAINER:?An explicitly selected disposable PostgreSQL container is required}"
: "${REVAER_LOCAL_DB_USER:?A test-service admin user is required}"
: "${REVAER_LOCAL_DB_PASSWORD:?A test-service admin password is required}"
if [[ "${operation}" == init && ! "${REVAER_TEST_RUNTIME_PASSWORD:-}" =~ ^[0-9a-f]{64}$ ]]; then
    echo "A 64-hex-character temporary runtime password is required." >&2
    exit 1
fi

source .github/build-inputs.env
actual_image="$(docker inspect --format '{{.Config.Image}}' "${PG_CONTAINER}")"
if [[ "${actual_image}" != "${POSTGRES_REBASELINE_IMAGE}" ]]; then
    echo "Test database container does not use the pinned PostgreSQL image." >&2
    exit 1
fi

scratch="/tmp/${name}-init"
scratch_created=0
database_created=0
roles_created=0
completed=0
export PGPASSWORD="${REVAER_LOCAL_DB_PASSWORD}"
export REVAER_TEST_RUNTIME_PASSWORD

psql_admin() {
    docker exec -i -e PGPASSWORD -e REVAER_TEST_RUNTIME_PASSWORD "${PG_CONTAINER}" \
        psql -X -q -h 127.0.0.1 -U "${REVAER_LOCAL_DB_USER}" \
        -v ON_ERROR_STOP=1 -v VERBOSITY=terse -v SHOW_CONTEXT=never \
        -v "database_name=${name}" "$@" >/dev/null
}

cleanup() {
    local status=$?
    trap - EXIT
    if [[ "${operation}" == init && "${database_created}" == 1 && "${completed}" == 0 ]]; then
        if ! psql_admin -d postgres -v verify_owner=false -v "drop_roles=${roles_created}" \
            -f "${scratch}/database-e2e-drop.sql"; then
            echo "Owned test database cleanup failed." >&2
            status=1
        fi
    fi
    if [[ "${scratch_created}" == 1 ]]; then
        if ! docker exec "${PG_CONTAINER}" rm -r -- "${scratch}"; then
            echo "Owned test SQL staging cleanup failed." >&2
            status=1
        fi
    fi
    exit "${status}"
}
trap cleanup EXIT

docker exec "${PG_CONTAINER}" mkdir -- "${scratch}"
scratch_created=1
for file in database-e2e-create.sql database-e2e-roles.sql database-e2e-seal.sql \
    database-e2e-drop.sql database-runtime-fixture-roles.sql database-runtime-fixture-owner-disable.sql; do
    docker cp "scripts/tests/${file}" "${PG_CONTAINER}:${scratch}/${file}"
done

if [[ "${operation}" == drop ]]; then
    psql_admin -d postgres -v verify_owner=true -v drop_roles=true -f "${scratch}/database-e2e-drop.sql"
    completed=1
    exit 0
fi

docker cp crates/revaer-data/init.sql "${PG_CONTAINER}:${scratch}/init.sql"
if command -v sha256sum >/dev/null 2>&1; then
    digest_output="$(sha256sum crates/revaer-data/init.sql)"
else
    digest_output="$(shasum -a 256 crates/revaer-data/init.sql)"
fi
digest="${digest_output%% *}"
[[ "${digest}" =~ ^[0-9a-f]{64}$ ]]
psql_admin -d postgres -f "${scratch}/database-e2e-create.sql"
database_created=1
psql_admin -d "${name}" -v "roles_file=${scratch}/database-runtime-fixture-roles.sql" \
    -f "${scratch}/database-e2e-roles.sql"
roles_created=1
PGPASSWORD="${REVAER_TEST_RUNTIME_PASSWORD}" docker exec -i -e PGPASSWORD "${PG_CONTAINER}" \
    psql -X -q -h 127.0.0.1 -U "${name}_owner" -d "${name}" \
    -v ON_ERROR_STOP=1 -v VERBOSITY=terse -v SHOW_CONTEXT=never \
    -v "init_file=${scratch}/init.sql" -v "init_digest=${digest}" \
    -v "runtime_role=${name}_runtime" -f "${scratch}/database-e2e-seal.sql" >/dev/null
psql_admin -d "${name}" -f "${scratch}/database-runtime-fixture-owner-disable.sql"
completed=1
