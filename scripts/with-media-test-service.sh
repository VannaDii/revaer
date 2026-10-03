#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
[[ "$#" -gt 0 ]] || { printf '%s\n' 'Provide a fixture consumer command' >&2; exit 1; }
source .github/build-inputs.env
: "${REVAER_OPERATOR_WORKFLOW_TOOLS_IMAGE:?Provide an immutable Linux tools image}"
: "${REVAER_OPERATOR_WORKFLOW_APP_EXECUTABLE:?Provide a current Linux app test executable relative to the repository}"
[[ "$REVAER_OPERATOR_WORKFLOW_TOOLS_IMAGE" =~ ^(sha256:[a-f0-9]{64}|[A-Za-z0-9./:_-]+@sha256:[a-f0-9]{64})$ ]]
[[ "$REVAER_OPERATOR_WORKFLOW_APP_EXECUTABLE" =~ ^target/[A-Za-z0-9_./-]+$ ]]
[[ "$REVAER_OPERATOR_WORKFLOW_APP_EXECUTABLE" != *..* ]]
[[ -x "$REVAER_OPERATOR_WORKFLOW_APP_EXECUTABLE" ]]
fixture_uid="$(id -u)"; fixture_gid="$(id -g)"
[[ "$fixture_uid" -ne 0 ]]
name="revaer_test_$(date +%s)_$$"
export PG_CONTAINER="revaer-root-positive-$$"
app="$PG_CONTAINER-app"; network="$PG_CONTAINER-net"; volume="$PG_CONTAINER-volume"
export REVAER_LOCAL_DB_USER=postgres REVAER_LOCAL_DB_PASSWORD="$(openssl rand -hex 32)" REVAER_TEST_RUNTIME_PASSWORD="$(openssl rand -hex 32)"
created=0
cleanup() {
 status=$?; trap - EXIT
 if docker inspect "$app" >/dev/null 2>&1; then
  docker logs "$app" 2>&1 | tail -n 18 || status=1
  docker rm -f "$app" >/dev/null || status=1
 fi
 if [[ "$created" == 1 ]]; then just db-test-drop "$name" || status=1; fi
 if docker inspect "$PG_CONTAINER" >/dev/null 2>&1; then docker rm -f "$PG_CONTAINER" >/dev/null || status=1; fi
 docker network rm "$network" >/dev/null || status=1
 docker volume rm "$volume" >/dev/null || status=1
 exit "$status"
}
trap cleanup EXIT
trap 'exit 143' TERM
trap 'exit 130' INT
docker network create "$network" >/dev/null
docker volume create "$volume" >/dev/null
docker run --rm --read-only --mount "type=volume,src=$volume,dst=/proof,volume-nocopy" "$REVAER_OPERATOR_WORKFLOW_TOOLS_IMAGE" bash -c "chown $fixture_uid:$fixture_gid /proof && chmod 700 /proof && stat -f -c PERSISTENCE_FIXTURE_FS:%T /proof"
docker run -d --name "$PG_CONTAINER" --network "$network" --network-alias postgres --tmpfs /var/lib/postgresql:rw -e POSTGRES_PASSWORD="$REVAER_LOCAL_DB_PASSWORD" "$POSTGRES_REBASELINE_IMAGE" >/dev/null
for attempt in {1..60}; do if docker exec "$PG_CONTAINER" pg_isready -U postgres >/dev/null 2>&1; then break; fi; sleep 1; done
just db-test-init "$name"; created=1
export DATABASE_URL="postgres://${name}_runtime:${REVAER_TEST_RUNTIME_PASSWORD}@postgres:5432/$name"
docker run -d --name "$app" --network "$network" --user "$fixture_uid:$fixture_gid" --read-only --cap-drop ALL --security-opt no-new-privileges --mount "type=volume,src=$volume,dst=/proof,volume-nocopy" --tmpfs /tmp:rw,mode=1777 -e DATABASE_URL -e "REVAER_OPERATOR_APP_BIN=/workspace/$REVAER_OPERATOR_WORKFLOW_APP_EXECUTABLE" -e HOME=/proof -e REVAER_E2E_SERVING_ENTRY=1 -e REVAER_MEDIA_WORKSPACE_ROOT=/proof/workspace -e REVAER_MEDIA_ROOT_CATALOG_FILE=/proof/catalog.json -v "$PWD:/workspace:ro" -w /tmp "$REVAER_OPERATOR_WORKFLOW_TOOLS_IMAGE" bash -c '
set -euo pipefail
stat -c OWNER:%u:%g:%a /proof
mkdir -p /proof/source /proof/source/Movies /proof/source/DryRun /proof/output /proof/workspace /proof/quarantine
chmod 700 /proof/workspace /proof/quarantine
if [[ ! -f /proof/source/original ]]; then
printf "original source bytes" >/proof/source/original
ffmpeg -hide_banner -loglevel error -nostdin -f lavfi -i testsrc2=size=160x90:rate=12:duration=1 -c:v ffv1 /proof/source/Movies/original.mkv
cp /proof/source/Movies/original.mkv /proof/source/DryRun/original.mkv
sha256sum /proof/source/Movies/original.mkv >/proof/source-before.sha256
fi
printf "%s" '"'"'{"format_version":1,"slots":[{"key":"quarantine","path":"/proof/quarantine","allowed_kinds":["quarantine"],"durability_class":"restart_persistent","durability_evidence":"linux_dedicated_mount","sole_writer_class":"revaer_exclusive","sole_writer_evidence":"linux_dedicated_service"},{"key":"source","path":"/proof/source","allowed_kinds":["output","source"],"durability_class":"restart_persistent","durability_evidence":"linux_dedicated_mount","sole_writer_class":"revaer_exclusive","sole_writer_evidence":"linux_dedicated_service"},{"key":"output","path":"/proof/output","allowed_kinds":["output"],"durability_class":"restart_persistent","durability_evidence":"linux_dedicated_mount","sole_writer_class":"revaer_exclusive","sole_writer_evidence":"linux_dedicated_service"},{"key":"workspace","path":"/proof/workspace","allowed_kinds":["workspace"],"durability_class":"restart_persistent","durability_evidence":"linux_dedicated_mount","sole_writer_class":"revaer_exclusive","sole_writer_evidence":"linux_dedicated_service"}]}'"'"' >/proof/catalog.json
binary="$REVAER_OPERATOR_APP_BIN"
"$binary" --exact bootstrap::runtime_tests::e2e_serving_entry --nocapture &
pid=$!
while [[ ! -f /proof/restart ]]; do kill -0 "$pid"; sleep 1; done
kill -TERM "$pid"
status=0; wait "$pid" || status=$?
[[ "$status" == 143 ]]
"$binary" --exact bootstrap::runtime_tests::e2e_serving_entry --nocapture &
pid=$!
printf "%s" "$pid" >/proof/restarted-pid
while true; do kill -0 "$pid"; sleep 1; done
' >/dev/null
export PGPASSWORD="$REVAER_TEST_RUNTIME_PASSWORD"
query() { docker exec -e PGPASSWORD "$PG_CONTAINER" psql -X -At -h 127.0.0.1 -U "${name}_runtime" -d "$name" -v ON_ERROR_STOP=1 -c "SELECT source_state,attestation_state,attestation_generation,root_kind,binding_ready_slot_count,destructive_ready_slot_count FROM media_root_catalog_readiness_get_v1()"; }
ready=0
for attempt in {1..60}; do
 rows="$(query)"
 if [[ "$rows" == ready\|ready\|* ]]; then ready=1; break; fi
 [[ "$(docker inspect -f "{{.State.Running}}" "$app")" == true ]] || break
 sleep 1
done
[[ "$ready" == 1 ]]
first="$rows"; printf "FIRST ACTIVATION\n%s\n" "$rows"
export APP_CONTAINER="$app"

"$@"
