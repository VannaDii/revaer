#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
renderer="${repo_root}/release/scripts/render-helm-annotations.sh"
test_root="$(mktemp -d)"

cleanup() {
    rm -rf "${test_root}"
    return 0
}
trap cleanup EXIT

chart_template="${test_root}/Chart.yaml"
annotations_file="${test_root}/annotations.yml"
rendered_chart="${test_root}/rendered-Chart.yaml"
expected_chart="${test_root}/expected-Chart.yaml"

cat > "${chart_template}" <<'EOF'
apiVersion: v2
name: revaer
annotations:
  artifacthub.io/license: MIT
  # __RELEASE_HELM_ANNOTATIONS__
  artifacthub.io/links: |
    - name: Documentation
      url: https://example.invalid/docs
EOF

cat > "${annotations_file}" <<'EOF'
  artifacthub.io/prerelease: "true"
  artifacthub.io/images: |
    - name: revaer
      image: 'ghcr.io/example/revaer:v1.2.3-rc.1'
      platforms:
        - linux/amd64
        - linux/arm64
  artifacthub.io/signKey: |
    fingerprint: '0123456789ABCDEF'
    url: 'https://example.invalid/revaer-helm-public.asc'
EOF

cat > "${expected_chart}" <<'EOF'
apiVersion: v2
name: revaer
annotations:
  artifacthub.io/license: MIT
  artifacthub.io/prerelease: "true"
  artifacthub.io/images: |
    - name: revaer
      image: 'ghcr.io/example/revaer:v1.2.3-rc.1'
      platforms:
        - linux/amd64
        - linux/arm64
  artifacthub.io/signKey: |
    fingerprint: '0123456789ABCDEF'
    url: 'https://example.invalid/revaer-helm-public.asc'
  artifacthub.io/links: |
    - name: Documentation
      url: https://example.invalid/docs
EOF

PATH="/usr/bin:/bin:${PATH}" bash "${renderer}" \
    "${chart_template}" \
    "${annotations_file}" \
    "${rendered_chart}"

if ! cmp -s "${expected_chart}" "${rendered_chart}"; then
    diff -u "${expected_chart}" "${rendered_chart}" >&2
    exit 1
fi

printf 'Helm multiline annotation rendering is portable\n'
