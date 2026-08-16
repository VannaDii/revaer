build: sync-assets
    cargo build --workspace --all-targets --all-features

build-release:
    cargo build --workspace --release --all-targets --all-features

release-artifacts: build-release api-export
    mkdir -p dist
    cp target/release/revaer-app dist/revaer-app
    sha256sum dist/revaer-app > dist/revaer-app.sha256
    cp docs/api/openapi.json dist/openapi.json

sbom:
    mkdir -p artifacts
    cargo metadata --format-version 1 --all-features --locked > artifacts/sbom.json

licenses:
    bash scripts/ensure-exact-cargo-tool.sh cargo-deny cargo-deny 0.18.9
    mkdir -p artifacts
    cargo deny list --format json > artifacts/licenses.json

api-export:
    cargo run -p revaer-api --bin generate_openapi

helm-annotation-test:
    bash release/tests/helm-annotation-rendering.sh

helm-lint: helm-annotation-test
    if ! command -v helm >/dev/null 2>&1; then \
        echo "helm is required to lint the chart"; \
        exit 1; \
    fi
    REVAER_HELM_SIGN=0 bash release/scripts/helm-package.sh 0.0.0-dev.0 v0.0.0-dev.0

helm-package chart_version app_version:
    bash release/scripts/helm-package.sh "{{chart_version}}" "{{app_version}}"

helm-publish chart_version app_version:
    bash release/scripts/helm-publish.sh "{{chart_version}}" "{{app_version}}"

release-dev:
    bash scripts/with-node.sh npm --prefix release ci
    bash scripts/with-node.sh node release/node_modules/.bin/semantic-release --extends ./release/release.config.js

release-lock:
    bash scripts/with-node.sh npm --prefix release install --package-lock-only
