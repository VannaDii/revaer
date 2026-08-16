#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root="$(mktemp -d "${TMPDIR:-/tmp}/revaer-sonar-installer.XXXXXX")"
trap 'rm -rf "${test_root}"' EXIT
mkdir -p "${test_root}/bin" "${test_root}/runner"

cat > "${test_root}/bin/curl" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
printf 'curl\n' >> "${MOCK_CALL_LOG}"
while [[ "$#" -gt 0 ]]; do
  if [[ "$1" = "--output" ]]; then
    output="$2"
    shift 2
  else
    shift
  fi
done
printf 'fixture\n' > "${output}"
MOCK

cat > "${test_root}/bin/gpg" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
fingerprint="679F1EE92B19609DE816FDE81DB198F93525EC1A"
printf 'gpg %s\n' "$*" >> "${MOCK_CALL_LOG}"
case " $* " in
  *" --with-colons "*) printf 'fpr:::::::::%s:\n' "${fingerprint}" ;;
  *" --verify "*)
    if [[ "${MOCK_GPG_VERIFY_EXIT:-0}" = "1" ]]; then
      exit 1
    fi
    if [[ "${MOCK_INVALID_SIGNATURE:-0}" = "1" ]]; then
      printf '[GNUPG:] BADSIG %s fixture\n' "${fingerprint}"
    else
      primary="${fingerprint}"
      if [[ "${MOCK_WRONG_PRIMARY:-0}" = "1" ]]; then
        primary="0000000000000000000000000000000000000000"
      fi
      printf '[GNUPG:] VALIDSIG D1436C0DBACEA48702AF97C363F1DD7753B8B315 2026-08-15 0 4 0 1 10 00 %s\n' \
        "${primary}"
    fi
    ;;
esac
MOCK

cat > "${test_root}/bin/unzip" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
while [[ "$#" -gt 0 ]]; do
  if [[ "$1" = "-d" ]]; then
    destination="$2"
    shift 2
  else
    shift
  fi
done
scanner="${destination}/sonar-scanner-8.1.0.6389-linux-x64/bin"
mkdir -p "${scanner}"
cat > "${scanner}/sonar-scanner" <<'SCANNER'
#!/usr/bin/env bash
printf 'SonarScanner CLI 8.1.0.6389\n'
SCANNER
chmod +x "${scanner}/sonar-scanner"
MOCK

cat > "${test_root}/bin/sha256sum" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${MOCK_INVALID_CHECKSUM:-0}" == "1" ]]; then
  digest="0000000000000000000000000000000000000000000000000000000000000000"
else
  digest="bb8f709f9cb73352f8d1260a3b3c506c0f41146754bc630762c126d795499d0b"
fi
printf '%s  %s\n' "${digest}" "$1"
MOCK
chmod +x "${test_root}/bin/curl" "${test_root}/bin/gpg" \
  "${test_root}/bin/sha256sum" "${test_root}/bin/unzip"

run_installer() {
  PATH="${test_root}/bin:${PATH}" \
  MOCK_CALL_LOG="${test_root}/calls.log" \
  RUNNER_TEMP="${test_root}/runner" \
  SONAR_SCANNER_FLAVOR="${SONAR_SCANNER_FLAVOR:-linux-x64}" \
  SONAR_SCANNER_INSTALL_ROOT="${test_root}/install" \
    bash "${repo_root}/scripts/install-sonar-scanner.sh"
}

bin_directory="$(run_installer)"
test -x "${bin_directory}/sonar-scanner"
[[ "$(grep -c '^curl$' "${test_root}/calls.log")" -eq 2 ]]
[[ "$(grep -c ' --verify ' "${test_root}/calls.log")" -eq 1 ]]

run_installer >/dev/null
[[ "$(grep -c '^curl$' "${test_root}/calls.log")" -eq 2 ]]
[[ "$(grep -c ' --verify ' "${test_root}/calls.log")" -eq 2 ]]

if MOCK_INVALID_CHECKSUM=1 run_installer >/dev/null 2>"${test_root}/checksum-error"; then
  echo "Sonar scanner installer accepted an invalid archive checksum" >&2
  exit 1
fi
grep -q 'archive SHA-256 mismatch for linux-x64' "${test_root}/checksum-error"

if MOCK_INVALID_SIGNATURE=1 run_installer >/dev/null 2>&1; then
  echo "Sonar scanner installer accepted an invalid archive signature" >&2
  exit 1
fi

if MOCK_WRONG_PRIMARY=1 run_installer >/dev/null 2>&1; then
  echo "Sonar scanner installer accepted a signature from the wrong primary key" >&2
  exit 1
fi

if MOCK_GPG_VERIFY_EXIT=1 run_installer >/dev/null 2>"${test_root}/verify-error"; then
  echo "Sonar scanner installer accepted a failed gpg verification command" >&2
  exit 1
fi
grep -q 'archive signature was not valid for the pinned SonarSource key' \
  "${test_root}/verify-error"

if SONAR_SCANNER_VERSION=../../outside run_installer >/dev/null 2>&1; then
  echo "Sonar scanner installer accepted traversal through SONAR_SCANNER_VERSION" >&2
  exit 1
fi
if SONAR_SCANNER_FLAVOR=../../outside run_installer >/dev/null 2>&1; then
  echo "Sonar scanner installer accepted traversal through SONAR_SCANNER_FLAVOR" >&2
  exit 1
fi

for digest in \
  bb8f709f9cb73352f8d1260a3b3c506c0f41146754bc630762c126d795499d0b \
  5e1c9328f4e261838de778c9e586ee608cca45ff7f0538108642219214628ba5 \
  8afc8bbff9008434e53b31cb681333ff643b999f84ca537db573d0fae8883cdc \
  20d12be4081896b337cd873d98ebd3d554be666086a45e31dd84a12ef51c3688; do
  grep -Fq "${digest}" "${repo_root}/scripts/install-sonar-scanner.sh"
done

gpg --batch --no-autostart --with-colons --show-keys \
  "${repo_root}/config/sonarsource-public-key.asc" \
  | awk -F: '$1 == "fpr" { print $10 }' \
  | grep -Fxq 679F1EE92B19609DE816FDE81DB198F93525EC1A
if grep -Eq 'keyserver|recv-keys' "${repo_root}/scripts/install-sonar-scanner.sh"; then
  echo "Sonar scanner installer still depends on a live keyserver" >&2
  exit 1
fi

printf '%s\n' "Sonar scanner installer tests passed"
