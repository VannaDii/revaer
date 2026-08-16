fmt:
    cargo fmt --all --check

fmt-fix:
    cargo fmt --all

policy:
    bash scripts/tests/policy-suite.sh

workflow-guardrails-test:
    bash scripts/tests/workflow-guardrails-test.sh

trivy-sarif-verify sarif_path:
    bash scripts/verify-trivy-sarif.sh "{{sarif_path}}"

trivy-sarif-policy-test:
    bash scripts/tests/trivy-sarif-policy-test.sh

image-compliance-generate image_reference package_inventory output_directory:
    bash scripts/generate-final-image-compliance-bundle.sh \
        "{{image_reference}}" \
        "{{package_inventory}}" \
        "{{output_directory}}"

image-compliance-validate bundle_path image_reference:
    bash scripts/validate-final-image-compliance-bundle.sh \
        "{{bundle_path}}" \
        "{{image_reference}}"

image-compliance-test:
    bash scripts/tests/final-image-compliance-test.sh

media-compliance-guardrails-test:
    bash scripts/tests/media-compliance-guardrails-test.sh

stack-check-contract-test:
    bash scripts/tests/stack-check-contract-test.sh

supply-chain-results-test:
    bash scripts/tests/supply-chain-results-test.sh

advisory-exception-guardrails-test:
    bash scripts/tests/advisory-exception-guardrails-test.sh

sonar-script-tests:
    bash scripts/tests/coverage-toolchain-test.sh
    bash scripts/tests/install-sonar-scanner-test.sh
    bash scripts/tests/prepare-sonar-scm-test.sh
    bash scripts/tests/sonar-inputs-test.sh
    bash scripts/tests/script-coverage-test.sh
    bash scripts/tests/sonar-scan-test.sh
    bash scripts/tests/sonar-result-guardrails-test.sh
    bash scripts/tests/supply-chain-results-test.sh

instruction-drift:
    bash scripts/instruction-drift-check.sh

lint: policy
    cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::cargo -W clippy::nursery -A clippy::multiple_crate_versions -A clippy::redundant_pub_crate
    cargo clippy --workspace --lib --bins --examples --all-features -- \
        -D warnings \
        -W clippy::expect_used \
        -W clippy::panic \
        -W clippy::todo \
        -W clippy::unimplemented \
        -W clippy::unreachable \
        -W clippy::unwrap_used

check:
    cargo --config 'build.rustflags=["-Dwarnings"]' check --workspace --all-targets --all-features

test:
    REVAER_TEST_DATABASE_URL="${REVAER_TEST_DATABASE_URL:-postgres://revaer:revaer@localhost:5432/postgres}" \
    DATABASE_URL="${DATABASE_URL:-$REVAER_TEST_DATABASE_URL}" \
        cargo --config 'build.rustflags=["-Dwarnings"]' test --workspace --all-features

test-native:
    REVAER_NATIVE_IT=1 \
    REVAER_TEST_DATABASE_URL="${REVAER_TEST_DATABASE_URL:-postgres://revaer:revaer@localhost:5432/postgres}" \
    DATABASE_URL="${DATABASE_URL:-$REVAER_TEST_DATABASE_URL}" \
        cargo --config 'build.rustflags=["-Dwarnings"]' test -p revaer-torrent-libt --all-features -- --test-threads=1

test-features-min:
    REVAER_TEST_DATABASE_URL="${REVAER_TEST_DATABASE_URL:-postgres://revaer:revaer@localhost:5432/postgres}" \
    DATABASE_URL="${DATABASE_URL:-$REVAER_TEST_DATABASE_URL}" \
        cargo --config 'build.rustflags=["-Dwarnings"]' test -p revaer-api --no-default-features
    REVAER_TEST_DATABASE_URL="${REVAER_TEST_DATABASE_URL:-postgres://revaer:revaer@localhost:5432/postgres}" \
    DATABASE_URL="${DATABASE_URL:-$REVAER_TEST_DATABASE_URL}" \
        cargo --config 'build.rustflags=["-Dwarnings"]' test -p revaer-app --no-default-features

udeps:
    set -euo pipefail; \
    required_udeps_version="0.1.57"; \
    requested_udeps_version="${REVAER_UDEPS_VERSION:-${required_udeps_version}}"; \
    if [ "${requested_udeps_version}" != "${required_udeps_version}" ]; then \
        echo "REVAER_UDEPS_VERSION must equal ${required_udeps_version}" >&2; \
        exit 1; \
    fi; \
    required_udeps_toolchain="nightly-2026-06-13"; \
    udeps_toolchain="${REVAER_UDEPS_TOOLCHAIN:-${required_udeps_toolchain}}"; \
    if [ "${udeps_toolchain}" != "${required_udeps_toolchain}" ]; then \
        echo "REVAER_UDEPS_TOOLCHAIN must equal ${required_udeps_toolchain}" >&2; \
        exit 1; \
    fi; \
    bash scripts/ensure-exact-cargo-tool.sh \
        cargo-udeps cargo-udeps "${requested_udeps_version}"; \
    if ! rustup run "${udeps_toolchain}" rustc --version >/dev/null 2>&1; then \
        rustup toolchain install "${udeps_toolchain}" --no-self-update; \
    fi; \
    mkdir -p target; \
    { \
        printf 'cargo-udeps-version='; \
        cargo udeps --version; \
        printf 'toolchain=%s\n' "${udeps_toolchain}"; \
        rustup run "${udeps_toolchain}" rustc --version --verbose; \
        printf 'command=cargo +%s udeps --workspace --all-targets\n' "${udeps_toolchain}"; \
    } | tee target/udeps-toolchain-evidence.txt; \
    cargo +"${udeps_toolchain}" udeps --workspace --all-targets

audit:
    set -euo pipefail; \
    required_audit_version="0.22.0"; \
    bash scripts/ensure-exact-cargo-tool.sh \
        cargo-audit cargo-audit "${required_audit_version}"; \
    ignore_args=""; \
    if [ -f .secignore ]; then \
        while IFS= read -r advisory; do \
            case "$advisory" in \
                \#*|"") ;; \
                *) ignore_args="$ignore_args --ignore $advisory" ;; \
            esac; \
        done < .secignore; \
    fi; \
    cargo audit --deny warnings $ignore_args

deny:
    required_deny_version="0.18.9"; \
    bash scripts/ensure-exact-cargo-tool.sh \
        cargo-deny cargo-deny "${required_deny_version}"
    cargo deny check

cov:
    bash scripts/ensure-exact-cargo-tool.sh \
        cargo-llvm-cov cargo-llvm-cov 0.8.7
    rustup component add llvm-tools-preview
    cargo llvm-cov clean --workspace
    toolchain_env="$(bash scripts/coverage-toolchain-env.sh)"; \
        eval "${toolchain_env}"; \
        export CC CXX LLVM_COV LLVM_PROFDATA REVAER_RUST_LLVM_VERSION; \
        test_database_url="${REVAER_TEST_DATABASE_URL:-$(bash scripts/local-postgres-url.sh postgres)}"; \
        database_url="${DATABASE_URL:-${test_database_url}}"; \
        db_managed="${REVAER_DB_MANAGED:-0}"; \
        if [ -z "${DATABASE_URL:-}" ]; then db_managed="1"; fi; \
        REVAER_DB_MANAGED="${db_managed}" REVAER_TEST_DATABASE_URL="${test_database_url}" DATABASE_URL="${database_url}" just db-start; \
        export REVAER_TEST_DATABASE_URL="${test_database_url}" DATABASE_URL="${database_url}"; \
        RUST_TEST_THREADS="${RUST_TEST_THREADS:-1}" \
        CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}" \
            cargo llvm-cov --workspace --all-features --include-ffi --no-report; \
        fail_list=""; \
        while IFS= read -r member; do \
            manifest="${member}/Cargo.toml"; \
            if [ ! -f "${manifest}" ]; then \
                continue; \
            fi; \
            if command -v rg >/dev/null 2>&1; then \
                name="$(rg -m1 '^name = "' "${manifest}" | sed -E 's/^name = "([^"]+)".*/\1/')"; \
            else \
                name="$(grep -m1 '^name = "' "${manifest}" | sed -E 's/^name = "([^"]+)".*/\1/')"; \
            fi; \
            if [ -z "${name}" ]; then \
                continue; \
            fi; \
            echo "== coverage: ${name} =="; \
            if ! cargo llvm-cov report --package "${name}" \
                --ignore-filename-regex '(^|/)(target|usr|opt|Applications)/|\.(c|cc|cpp|h|hpp|ipp)$' \
                --json --summary-only --fail-under-lines 90 >/dev/null; then \
                fail_list="${fail_list} ${name}"; \
            fi; \
        done < <(awk ' \
            /^members = \[/ { in_members=1; next } \
            in_members && /^]/ { in_members=0; next } \
            in_members && match($0, /"[^"]+"/) { print substr($0, RSTART + 1, RLENGTH - 2) } \
        ' Cargo.toml); \
        if [ -n "${fail_list}" ]; then \
            echo "Coverage below 90% for:${fail_list}"; \
            exit 1; \
        fi; \
        rm -rf coverage; \
        mkdir -p coverage; \
        cargo llvm-cov report \
            --ignore-filename-regex '(^|/)(target|usr|opt|Applications)/|\.(c|cc|cpp|h|hpp|ipp)$' \
            --lcov --output-path coverage/lcov.info; \
        cargo llvm-cov report \
            --ignore-filename-regex '(^|/)(target|usr|opt|Applications)/|\.(c|cc|cpp|h|hpp|ipp)$' \
            --html --output-dir coverage; \
        cargo llvm-cov report --text --output-path coverage/llvm-cov.txt
    just script-coverage

script-coverage:
    command -v kcov >/dev/null 2>&1 || { echo "kcov is required for authored shell coverage" >&2; exit 1; }
    rm -rf coverage/scripts
    mkdir -p coverage/scripts/ruby
    REVAER_RUBY_COVERAGE_DIR="${PWD}/coverage/scripts/ruby" \
    RUBYOPT="-r${PWD}/scripts/ruby-coverage-bootstrap.rb" \
        kcov --include-path="${PWD}/scripts" coverage/scripts/kcov scripts/tests/policy-suite.sh
    REVAER_RUBY_COVERAGE_DIR="${PWD}/coverage/scripts/ruby" \
    RUBYOPT="-r${PWD}/scripts/ruby-coverage-bootstrap.rb" \
        ruby scripts/generate-generic-coverage.rb \
            coverage/scripts/kcov coverage/scripts/ruby coverage/scripts/preliminary.xml
    ruby scripts/generate-generic-coverage.rb \
        coverage/scripts/kcov coverage/scripts/ruby coverage/script-coverage.xml
    rm coverage/scripts/preliminary.xml
    test -s coverage/script-coverage.xml
    grep -Eq "<file path=['\"]scripts/.*\.sh['\"]" coverage/script-coverage.xml
    grep -Eq "<file path=['\"]scripts/.*\.rb['\"]" coverage/script-coverage.xml
    grep -Eq "covered=['\"]true['\"]" coverage/script-coverage.xml
    grep -Eq "covered=['\"]false['\"]" coverage/script-coverage.xml

sonar-compile-db:
    mkdir -p coverage
    rm -f coverage/compile_commands.json
    cargo clean --target-dir "${PWD}/target/sonar-build" -p revaer-torrent-libt
    REVAER_NATIVE_IT=1 \
    CARGO_TARGET_DIR="${PWD}/target/sonar-build" \
    REVAER_NATIVE_COMPILE_COMMANDS_PATH="${PWD}/coverage/compile_commands.json" \
        cargo --config 'build.rustflags=["-Dwarnings"]' build -p revaer-torrent-libt --all-features
    test -s coverage/compile_commands.json

sonar-verify-inputs:
    bash scripts/verify-sonar-inputs.sh

sonar-prepare-sources:
    test -z "$(git ls-files -- tests/node_modules release/node_modules)"
    rm -rf tests/node_modules release/node_modules \
        crates/revaer-ui/dist-serve tests/test-results tests/playwright-report \
        tests/support/api/schema.ts

sonar-prepare-scm:
    bash scripts/prepare-sonar-scm.sh

sonar-scan:
    bash scripts/sonar-scan.sh

sonar-package-report:
    test -s artifacts/sonar/scanner.log
    test -s .scannerwork/report-task.txt
    test -d .scannerwork/scanner-report
    tar -cJf .scannerwork/scanner-report.tar.xz -C .scannerwork scanner-report
    test -s .scannerwork/scanner-report.tar.xz
    tar -tf .scannerwork/scanner-report.tar.xz | grep -q '^scanner-report/'

sonar-verify-result: sonar-package-report
    bash scripts/sonar-result-guardrails.sh

verify-supply-chain-results:
    bash scripts/verify-supply-chain-results.sh

js-release-coverage:
    bash scripts/with-node.sh npm --prefix tests ci --ignore-scripts
    bash scripts/with-node.sh npm --prefix tests audit --audit-level=info
    rm -rf coverage/js/release
    mkdir -p coverage/js/release/config coverage/js/release/stub
    bash scripts/with-node.sh tests/node_modules/.bin/c8 \
        --reporter=lcovonly \
        --reports-dir coverage/js/release/config \
        --include release/release.config.js \
        --include scripts/js-coverage/release-config-smoke.cjs \
        node scripts/js-coverage/release-config-smoke.cjs
    bash scripts/with-node.sh tests/node_modules/.bin/c8 \
        --reporter=lcovonly \
        --reports-dir coverage/js/release/stub \
        --include scripts/js-coverage/semantic-release-npm-stub.mjs \
        --include vendor/semantic-release-npm-stub/index.js \
        node scripts/js-coverage/semantic-release-npm-stub.mjs
    cat coverage/js/release/config/lcov.info coverage/js/release/stub/lcov.info \
        > coverage/js/release/lcov.info
    rm -rf coverage/js/release/config coverage/js/release/stub
    test -s coverage/js/release/lcov.info
    grep -q '^SF:release/release.config.js' coverage/js/release/lcov.info
    grep -q '^SF:vendor/semantic-release-npm-stub/index.js' coverage/js/release/lcov.info
    grep -q '^DA:' coverage/js/release/lcov.info

js-coverage-merge:
    mkdir -p coverage
    lcov_files="$(find coverage/js -name lcov.info -type f | sort)"; \
    if [ -z "${lcov_files}" ]; then \
        echo "No JavaScript/TypeScript LCOV files were produced." >&2; \
        exit 1; \
    fi; \
    cat ${lcov_files} > coverage/js-lcov.info
    test -s coverage/js-lcov.info
    grep -q '^SF:' coverage/js-lcov.info
    grep -q '^DA:' coverage/js-lcov.info

validate:
    REVAER_TEST_DATABASE_URL="${REVAER_TEST_DATABASE_URL:-postgres://revaer:revaer@localhost:5432/postgres}"
    DATABASE_URL="${DATABASE_URL:-$REVAER_TEST_DATABASE_URL}"
    export REVAER_TEST_DATABASE_URL DATABASE_URL
    just db-start
    just fmt lint helm-lint instruction-drift check-assets udeps audit deny ui-build test test-features-min cov

ci: validate
    just build-release
