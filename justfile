set shell := ["bash", "-c"]

# Compatibility only: rv owns every implementation and gate.

db-pristine-catalog-generate *args:
    uv run --locked -- rv db-pristine-catalog-generate {{args}}

db-pristine-catalog-validate *args:
    uv run --locked -- rv db-pristine-catalog-validate {{args}}

db-pristine-catalog-test *args:
    uv run --locked -- rv db-pristine-catalog-test {{args}}

db-rebaseline-freeze *args:
    uv run --locked -- rv db-rebaseline-freeze {{args}}

db-rebaseline-candidate *args:
    uv run --locked -- rv db-rebaseline-candidate {{args}}

db-init-prefix-check *args:
    uv run --locked -- rv db-init-prefix-check {{args}}

db-init-finalize *args:
    uv run --locked -- rv db-init-finalize {{args}}

test-database-baseline-read *args:
    uv run --locked -- rv test-database-baseline-read {{args}}

db-init-pool-probe *args:
    uv run --locked -- rv db-init-pool-probe {{args}}

db-init-cancellation-probe *args:
    uv run --locked -- rv db-init-cancellation-probe {{args}}

stack-changed-lines *args:
    uv run --locked -- rv stack-changed-lines {{args}}

db-init-assembly-changed-lines *args:
    uv run --locked -- rv db-init-assembly-changed-lines {{args}}

dev *args:
    uv run --locked -- rv dev {{args}}

zombies *args:
    uv run --locked -- rv zombies {{args}}

download-test-fixtures *args:
    uv run --locked -- rv download-test-fixtures {{args}}

generate-test-fixtures *args:
    uv run --locked -- rv generate-test-fixtures {{args}}

verify-test-fixtures *args:
    uv run --locked -- rv verify-test-fixtures {{args}}

update-test-fixture-probes *args:
    uv run --locked -- rv update-test-fixture-probes {{args}}

test-media-conversion *args:
    uv run --locked -- rv test-media-conversion {{args}}

test-media-root-catalog *args:
    uv run --locked -- rv test-media-root-catalog {{args}}

test-media-root-contract *args:
    uv run --locked -- rv test-media-root-contract {{args}}

test-media-broker-codec *args:
    uv run --locked -- rv test-media-broker-codec {{args}}

clean-test-fixtures *args:
    uv run --locked -- rv clean-test-fixtures {{args}}

clean-test-media *args:
    uv run --locked -- rv clean-test-media {{args}}

test-fixture-scripts *args:
    uv run --locked -- rv test-fixture-scripts {{args}}

fixture-cache-key *args:
    uv run --locked -- rv fixture-cache-key {{args}}

workflow-metadata *args:
    uv run --locked -- rv workflow-metadata {{args}}

workflow-chart-versions *args:
    uv run --locked -- rv workflow-chart-versions {{args}}

workflow-matrix *args:
    uv run --locked -- rv workflow-matrix {{args}}

workflow-report *args:
    uv run --locked -- rv workflow-report {{args}}

verify-supply-chain-results *args:
    uv run --locked -- rv verify-supply-chain-results {{args}}

policy *args:
    uv run --locked -- rv policy {{args}}

lint *args:
    uv run --locked -- rv lint {{args}}

instruction-drift *args:
    uv run --locked -- rv instruction-drift {{args}}

validate *args:
    uv run --locked -- rv validate {{args}}

ci *args:
    uv run --locked -- rv ci {{args}}

lock *args:
    uv run --locked -- rv lock {{args}}

release-lock *args:
    uv run --locked -- rv release-lock {{args}}

helm-lint *args:
    uv run --locked -- rv helm-lint {{args}}

helm-annotation-test *args:
    uv run --locked -- rv helm-annotation-test {{args}}

helm-package-test *args:
    uv run --locked -- rv helm-package-test {{args}}

compliance-chart-test *args:
    uv run --locked -- rv compliance-chart-test {{args}}

helm-package *args:
    uv run --locked -- rv helm-package {{args}}

helm-publish *args:
    uv run --locked -- rv helm-publish {{args}}

helm-verify *args:
    uv run --locked -- rv helm-verify {{args}}

setup *args:
    uv run --locked -- rv setup {{args}}

doctor *args:
    uv run --locked -- rv doctor {{args}}

fmt *args:
    uv run --locked -- rv fmt {{args}}

fmt-fix *args:
    uv run --locked -- rv fmt-fix {{args}}

tooling-check *args:
    uv run --locked -- rv tooling-check {{args}}

tooling-audit *args:
    uv run --locked -- rv tooling-audit {{args}}

tooling-cov *args:
    uv run --locked -- rv tooling-cov {{args}}

script-coverage *args:
    uv run --locked -- rv script-coverage {{args}}

check *args:
    uv run --locked -- rv check {{args}}

test *args:
    uv run --locked -- rv test {{args}}

test-native *args:
    uv run --locked -- rv test-native {{args}}

test-runtime-shutdown *args:
    uv run --locked -- rv test-runtime-shutdown {{args}}

lint-runtime-shutdown *args:
    uv run --locked -- rv lint-runtime-shutdown {{args}}

test-features-min *args:
    uv run --locked -- rv test-features-min {{args}}

db-migrate *args:
    uv run --locked -- rv db-migrate {{args}}

db-test-init *args:
    uv run --locked -- rv db-test-init {{args}}

db-test-drop *args:
    uv run --locked -- rv db-test-drop {{args}}

db-start *args:
    uv run --locked -- rv db-start {{args}}

db-reset *args:
    uv run --locked -- rv db-reset {{args}}

db-seed *args:
    uv run --locked -- rv db-seed {{args}}

cov *args:
    uv run --locked -- rv cov {{args}}

cov-report *args:
    uv run --locked -- rv cov-report {{args}}

sonar-compile-db *args:
    uv run --locked -- rv sonar-compile-db {{args}}

sonar-scan *args:
    uv run --locked -- rv sonar-scan {{args}}

sonar-verify-inputs *args:
    uv run --locked -- rv sonar-verify-inputs {{args}}

sonar-prepare-scm *args:
    uv run --locked -- rv sonar-prepare-scm {{args}}

sonar-prepare-sources *args:
    uv run --locked -- rv sonar-prepare-sources {{args}}

python-coverage-merge *args:
    uv run --locked -- rv python-coverage-merge {{args}}

js-coverage-merge *args:
    uv run --locked -- rv js-coverage-merge {{args}}

sonar-package-report *args:
    uv run --locked -- rv sonar-package-report {{args}}

sonar-verify-result *args:
    uv run --locked -- rv sonar-verify-result {{args}}

docker-build *args:
    uv run --locked -- rv docker-build {{args}}

docker-scan *args:
    uv run --locked -- rv docker-scan {{args}}

trivy-sarif-verify *args:
    uv run --locked -- rv trivy-sarif-verify {{args}}

image-compliance-generate *args:
    uv run --locked -- rv image-compliance-generate {{args}}

image-compliance-validate *args:
    uv run --locked -- rv image-compliance-validate {{args}}

media-compliance-guardrails *args:
    uv run --locked -- rv media-compliance-guardrails {{args}}

image-build-push *args:
    uv run --locked -- rv image-build-push {{args}}

image-build-verify *args:
    uv run --locked -- rv image-build-verify {{args}}

image-inventory *args:
    uv run --locked -- rv image-inventory {{args}}

image-scan *args:
    uv run --locked -- rv image-scan {{args}}

image-sign-attest *args:
    uv run --locked -- rv image-sign-attest {{args}}

image-attestation-verify *args:
    uv run --locked -- rv image-attestation-verify {{args}}

image-manifest-create *args:
    uv run --locked -- rv image-manifest-create {{args}}

image-manifest-verify *args:
    uv run --locked -- rv image-manifest-verify {{args}}

image-manifest-sign *args:
    uv run --locked -- rv image-manifest-sign {{args}}

image-scan-category *args:
    uv run --locked -- rv image-scan-category {{args}}

container-build *args:
    uv run --locked -- rv container-build {{args}}

container-runtime *args:
    uv run --locked -- rv container-runtime {{args}}

sync-assets *args:
    uv run --locked -- rv sync-assets {{args}}

check-assets *args:
    uv run --locked -- rv check-assets {{args}}

build *args:
    uv run --locked -- rv build {{args}}

build-release *args:
    uv run --locked -- rv build-release {{args}}

api-export *args:
    uv run --locked -- rv api-export {{args}}

release-artifacts *args:
    uv run --locked -- rv release-artifacts {{args}}

audit *args:
    uv run --locked -- rv audit {{args}}

deny *args:
    uv run --locked -- rv deny {{args}}

udeps *args:
    uv run --locked -- rv udeps {{args}}

sbom *args:
    uv run --locked -- rv sbom {{args}}

licenses *args:
    uv run --locked -- rv licenses {{args}}

ui-build *args:
    uv run --locked -- rv ui-build {{args}}

ui-serve *args:
    uv run --locked -- rv ui-serve {{args}}

ui-e2e-app-test *args:
    uv run --locked -- rv ui-e2e-app-test {{args}}

ui-e2e *args:
    uv run --locked -- rv ui-e2e {{args}}

ui-e2e-coverage *args:
    uv run --locked -- rv ui-e2e-coverage {{args}}

ui-e2e-shard-coverage *args:
    uv run --locked -- rv ui-e2e-shard-coverage {{args}}

runbook *args:
    uv run --locked -- rv runbook {{args}}

docs *args:
    uv run --locked -- rv docs {{args}}

docs-install *args:
    uv run --locked -- rv docs-install {{args}}

docs-build *args:
    uv run --locked -- rv docs-build {{args}}

docs-serve *args:
    uv run --locked -- rv docs-serve {{args}}

docs-index *args:
    uv run --locked -- rv docs-index {{args}}

docs-link-check *args:
    uv run --locked -- rv docs-link-check {{args}}

docs-guard *args:
    uv run --locked -- rv docs-guard {{args}}

docs-prepare *args:
    uv run --locked -- rv docs-prepare {{args}}
