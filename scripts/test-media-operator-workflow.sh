#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
bash scripts/with-media-test-service.sh \
    bash scripts/with-node.sh tests/node_modules/.bin/c8 \
    --reporter=lcovonly --reports-dir coverage/js/media-operator \
    --include tests/support/media-operator-workflow.cjs \
    --include tests/support/media-service-relay.cjs --exclude="" \
    node tests/support/media-operator-workflow.cjs
test -s coverage/js/media-operator/lcov.info
grep -Eq '^SF:tests/support/media-operator-workflow.cjs$' coverage/js/media-operator/lcov.info
grep -Eq '^DA:[0-9]+,[1-9][0-9]*' coverage/js/media-operator/lcov.info
