#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
expected_scanner_version="8.1.0.6389"
scanner_version="${SONAR_SCANNER_VERSION:-${expected_scanner_version}}"
scanner_base_url="${SONAR_SCANNER_BINARIES_URL:-https://binaries.sonarsource.com/Distribution/sonar-scanner-cli}"
key_fingerprint="679F1EE92B19609DE816FDE81DB198F93525EC1A"
public_key_path="${repo_root}/config/sonarsource-public-key.asc"
curl_command="${CURL_COMMAND:-curl}"
gpg_command="${GPG_COMMAND:-gpg}"
unzip_command="${UNZIP_COMMAND:-unzip}"

for command in "${curl_command}" "${gpg_command}" "${unzip_command}"; do
  if ! command -v "${command}" >/dev/null 2>&1; then
    printf 'Required Sonar scanner installer command is unavailable: %s\n' "${command}" >&2
    exit 1
  fi
done

if [[ "${scanner_version}" != "${expected_scanner_version}" ||
  ! "${scanner_version}" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  printf 'Sonar scanner version must be exactly %s\n' "${expected_scanner_version}" >&2
  exit 1
fi

if [[ ! -s "${public_key_path}" ]]; then
  echo "Committed SonarSource public key is unavailable" >&2
  exit 1
fi

if [[ -n "${SONAR_SCANNER_FLAVOR:-}" ]]; then
  flavor="${SONAR_SCANNER_FLAVOR}"
else
  case "$(uname -s):$(uname -m)" in
    Linux:x86_64) flavor="linux-x64" ;;
    Linux:aarch64|Linux:arm64) flavor="linux-aarch64" ;;
    Darwin:x86_64) flavor="macosx-x64" ;;
    Darwin:arm64) flavor="macosx-aarch64" ;;
    *)
      printf 'Unsupported Sonar scanner platform: %s:%s\n' "$(uname -s)" "$(uname -m)" >&2
      exit 1
      ;;
  esac
fi

case "${flavor}" in
  linux-x64)
    expected_archive_sha256="bb8f709f9cb73352f8d1260a3b3c506c0f41146754bc630762c126d795499d0b"
    ;;
  linux-aarch64)
    expected_archive_sha256="5e1c9328f4e261838de778c9e586ee608cca45ff7f0538108642219214628ba5"
    ;;
  macosx-x64)
    expected_archive_sha256="8afc8bbff9008434e53b31cb681333ff643b999f84ca537db573d0fae8883cdc"
    ;;
  macosx-aarch64)
    expected_archive_sha256="20d12be4081896b337cd873d98ebd3d554be666086a45e31dd84a12ef51c3688"
    ;;
  *)
    printf 'Unsupported Sonar scanner flavor: %s\n' "${flavor}" >&2
    exit 1
    ;;
esac

install_root="${SONAR_SCANNER_INSTALL_ROOT:-${HOME}/.local/revaer/sonar-scanner}/${scanner_version}/${flavor}"
archive_name="sonar-scanner-cli-${scanner_version}-${flavor}.zip"
archive_path="${install_root}/${archive_name}"
signature_path="${archive_path}.asc"
scanner_directory="${install_root}/sonar-scanner-${scanner_version}-${flavor}"
download_url="${scanner_base_url%/}/${archive_name}"

mkdir -p "${install_root}"
if [[ ! -s "${archive_path}" ]]; then
  "${curl_command}" --fail --location --proto '=https' --tlsv1.2 \
    --output "${archive_path}" "${download_url}"
fi
if [[ ! -s "${signature_path}" ]]; then
  "${curl_command}" --fail --location --proto '=https' --tlsv1.2 \
    --output "${signature_path}" "${download_url}.asc"
fi

if command -v sha256sum >/dev/null 2>&1; then
  actual_archive_sha256="$(sha256sum "${archive_path}" | awk '{ print $1 }')"
elif command -v shasum >/dev/null 2>&1; then
  actual_archive_sha256="$(shasum -a 256 "${archive_path}" | awk '{ print $1 }')"
else
  echo "A SHA-256 implementation is required to verify the Sonar scanner archive" >&2
  exit 1
fi
if [[ "${actual_archive_sha256}" != "${expected_archive_sha256}" ]]; then
  printf 'Sonar scanner archive SHA-256 mismatch for %s: expected %s, found %s\n' \
    "${flavor}" "${expected_archive_sha256}" "${actual_archive_sha256}" >&2
  exit 1
fi

gpg_temp_base="${RUNNER_TEMP:-${TMPDIR:-/tmp}}"
gpg_socket_probe="${gpg_temp_base%/}/revaer-sonar-gpg.XXXXXX/S.gpg-agent.browser"
if (( ${#gpg_socket_probe} > 107 )); then
  gpg_temp_base="/tmp"
fi
gpg_home="$(mktemp -d "${gpg_temp_base%/}/revaer-sonar-gpg.XXXXXX")"
extract_root="$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/revaer-sonar-extract.XXXXXX")"
cleanup() {
  rm -rf "${gpg_home}" "${extract_root}"
}
trap cleanup EXIT
chmod 700 "${gpg_home}"

"${gpg_command}" --homedir "${gpg_home}" --batch --no-autostart \
  --import "${public_key_path}" >/dev/null

imported_fingerprints="$(
  "${gpg_command}" --homedir "${gpg_home}" --batch --no-autostart --with-colons \
    --fingerprint "${key_fingerprint}" | awk -F: '$1 == "fpr" { print $10 }'
)"
if ! grep -Fxq "${key_fingerprint}" <<<"${imported_fingerprints}"; then
  echo "Imported SonarSource key fingerprint did not match the pinned fingerprint" >&2
  exit 1
fi

set +e
verification_output="$(
  "${gpg_command}" --homedir "${gpg_home}" --batch --no-autostart --status-fd 1 \
    --verify "${signature_path}" "${archive_path}" 2>&1
)"
verification_status=$?
set -e
if [[ "${verification_status}" -ne 0 ]] ||
  ! awk -v primary_fingerprint="${key_fingerprint}" '
      $1 == "[GNUPG:]" && $2 == "VALIDSIG" && $NF == primary_fingerprint { valid = 1 }
      END { exit(valid ? 0 : 1) }
    ' <<<"${verification_output}"; then
  echo "Sonar scanner archive signature was not valid for the pinned SonarSource key" >&2
  exit 1
fi

"${unzip_command}" -q "${archive_path}" -d "${extract_root}"
extracted_scanner="${extract_root}/sonar-scanner-${scanner_version}-${flavor}"
if [[ ! -x "${extracted_scanner}/bin/sonar-scanner" ]]; then
  echo "Verified Sonar scanner archive did not contain the expected executable" >&2
  exit 1
fi
rm -rf "${scanner_directory}"
mv "${extracted_scanner}" "${scanner_directory}"

version_output="$("${scanner_directory}/bin/sonar-scanner" --version 2>&1)"
if ! grep -Fq "SonarScanner CLI ${scanner_version}" <<<"${version_output}"; then
  printf 'Expected SonarScanner CLI %s, received:\n%s\n' \
    "${scanner_version}" "${version_output}" >&2
  exit 1
fi

printf '%s\n' "${scanner_directory}/bin"
