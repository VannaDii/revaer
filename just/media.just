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

test-media-conversion:
    #!/usr/bin/env bash
    set -euo pipefail
    report_path="${REVAER_MEDIA_CONVERSION_REPORT:-target/media-conversion-report.md}"
    rm -f -- "${report_path}"
    REVAER_MEDIA_CONVERSION_REPORT="${report_path}.preparation" just verify-test-fixtures
    cargo --config 'build.rustflags=["-Dwarnings"]' test \
        -p revaer-media-runtime --all-features --test media_fixtures -- --include-ignored
    test -s "${report_path}"
    grep -Fxq -- '- Outcome: passed' "${report_path}"
    grep -Eq -- '^- Pipeline actions: [1-9][0-9]*$' "${report_path}"
    grep -Eq -- '^- Video transcodes: [1-9][0-9]*$' "${report_path}"
    grep -Eq -- '^- Audio transcodes: [1-9][0-9]*$' "${report_path}"
    grep -Fxq -- '- Pipeline failures: 0' "${report_path}"
    grep -Fxq -- '- Suite failures: 0' "${report_path}"

test-media-root-catalog:
    cargo --config 'build.rustflags=["-Dwarnings"]' test \
        -p revaer-media-runtime --all-features root_catalog

test-media-root-contract:
    cargo --config 'build.rustflags=["-Dwarnings"]' test \
        -p revaer-api-models --all-features media_root_contract

test-media-broker-codec:
    cargo --config 'build.rustflags=["-Dwarnings"]' test \
        -p revaer-media-runtime --all-features process::broker

update-test-fixture-probes:
    bash scripts/test-fixtures/update-probe-snapshots.sh

clean-test-fixtures:
    bash scripts/test-fixtures/clean-test-fixtures.sh

test-media-operator-workflow:
    bash scripts/test-media-operator-workflow.sh

clean-test-media:
    find "${TMPDIR:-/tmp}" -maxdepth 1 -type d -name 'revaer-media-conversion.*' \
        -exec rm -rf {} +
