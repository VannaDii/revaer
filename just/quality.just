fmt:
    cargo fmt --all --check

fmt-fix:
    cargo fmt --all

policy:
    bash scripts/tests/policy-suite.sh
    just test-fixture-scripts

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
