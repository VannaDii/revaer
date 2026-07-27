#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

jq -s -e '
  .[0] as $lock |
  .[1] as $manifest |
  ($lock.schemaVersion == 1) and
  ($lock.cacheKeyInputs | index("test-fixtures/lock.json") != null) and
  ($lock.upstreams.chromium.revision | test("^[0-9a-f]{40}$")) and
  ($lock.upstreams.matroska.revision | test("^[0-9a-f]{40}$")) and
  ($lock.sources | length > 0) and
  ([$lock.sources[].id] | length == (unique | length)) and
  ([$lock.sources[].path] | length == (unique | length)) and
  all($manifest.fixtures[];
    ((has("allowProbeDiagnostics") | not) or
      (.allowProbeDiagnostics | type == "boolean"))) and
  ([$manifest.fixtures[] | select(.allowProbeDiagnostics == true) | .id] | sort) ==
    ["chromium-bear-1280x720-av-frag-mp4"] and
  all($lock.sources[];
    (.id | type == "string" and length > 0) and
    (.path | type == "string" and startswith("test-fixtures/")) and
    (.encoding == "raw" or .encoding == "base64") and
    (.sha256 | test("^[0-9a-f]{64}$")) and
    (.minimumBytes | type == "number" and . > 0) and
    (.maximumBytes == .minimumBytes) and
    (.urls | type == "array" and length > 0) and
    all(.urls[];
      type == "string" and
      startswith("https://") and
      (contains("/+/lkgr/") | not) and
      (contains("/blob/master/") | not))) and
  ([$lock.sources[] | {id, path}] | sort_by(.id)) ==
    ([$manifest.fixtures[] | select(.shouldDownload) | {id, path}] | sort_by(.id))
' test-fixtures/lock.json test-fixtures/manifest.json >/dev/null

printf 'test-lock-manifest: immutable revisions, integrity bounds, diagnostic policy, cache key, and manifest alignment passed\n'
