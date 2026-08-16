test-fixture-scripts:
    bash scripts/test-fixtures/test-lock-manifest.sh
    bash scripts/test-fixtures/test-download-integrity.sh
    bash scripts/test-fixtures/test-probe-verification.sh

download-test-fixtures:
    bash scripts/test-fixtures/download-test-fixtures.sh

generate-test-fixtures:
    bash scripts/test-fixtures/generate-derived-fixtures.sh

verify-test-fixtures:
    bash scripts/test-fixtures/verify-fixtures.sh

test-media-conversion: verify-test-fixtures

update-test-fixture-probes:
    bash scripts/test-fixtures/update-probe-snapshots.sh

clean-test-fixtures:
    bash scripts/test-fixtures/clean-test-fixtures.sh

clean-test-media:
    find "${TMPDIR:-/tmp}" -maxdepth 1 -type d -name 'revaer-media-conversion.*' \
        -exec rm -rf {} +
