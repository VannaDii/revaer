#!/usr/bin/env bash

fixture_sha256() {
  local path="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "${path}" | awk '{print $1}'
    return
  fi
  shasum -a 256 "${path}" | awk '{print $1}'
}

fixture_size() {
  local path="$1"
  wc -c <"${path}" | tr -d '[:space:]'
}

fixture_validate() {
  local id="$1"
  local path="$2"
  local expected_sha256="$3"
  local minimum_bytes="$4"
  local maximum_bytes="$5"

  if [[ -L "${path}" ]]; then
    printf 'fixture-integrity: %s must not be a symbolic link: %s\n' "${id}" "${path}" >&2
    return 1
  fi
  if [[ ! -f "${path}" ]]; then
    printf 'fixture-integrity: %s is missing: %s\n' "${id}" "${path}" >&2
    return 1
  fi

  local actual_bytes
  actual_bytes="$(fixture_size "${path}")"
  if ((actual_bytes < minimum_bytes || actual_bytes > maximum_bytes)); then
    printf 'fixture-integrity: %s has %s bytes; expected %s..%s\n' \
      "${id}" "${actual_bytes}" "${minimum_bytes}" "${maximum_bytes}" >&2
    return 1
  fi

  local actual_sha256
  actual_sha256="$(fixture_sha256 "${path}")"
  if [[ "${actual_sha256}" != "${expected_sha256}" ]]; then
    printf 'fixture-integrity: %s SHA-256 mismatch: expected %s, received %s\n' \
      "${id}" "${expected_sha256}" "${actual_sha256}" >&2
    return 1
  fi
}

fixture_decode_base64() {
  local source_path="$1"
  local destination="$2"

  if base64 --decode <"${source_path}" >"${destination}" 2>/dev/null; then
    return 0
  fi
  base64 -D <"${source_path}" >"${destination}" 2>/dev/null
}

fixture_acquire() {
  local id="$1"
  local destination="$2"
  local encoding="$3"
  local expected_sha256="$4"
  local minimum_bytes="$5"
  local maximum_bytes="$6"
  local temporary_root="$7"
  shift 7

  if [[ -e "${destination}" && "${REVAER_FIXTURE_FORCE_DOWNLOAD:-0}" != "1" ]]; then
    if fixture_validate "${id}" "${destination}" "${expected_sha256}" "${minimum_bytes}" "${maximum_bytes}"; then
      printf 'download-test-fixtures: verified cached %s: %s\n' "${id}" "${destination}"
      return 0
    fi
    printf 'download-test-fixtures: discarding invalid cached %s\n' "${id}" >&2
    rm -f "${destination}"
  fi

  local curl_bin="${REVAER_FIXTURE_CURL_BIN:-curl}"
  local connect_timeout="${REVAER_FIXTURE_CONNECT_TIMEOUT_SECONDS:-15}"
  local deadline="${REVAER_FIXTURE_DEADLINE_SECONDS:-120}"
  if [[ ! "${minimum_bytes}" =~ ^[0-9]+$ || ! "${maximum_bytes}" =~ ^[0-9]+$ ]] ||
    ((minimum_bytes == 0 || maximum_bytes < minimum_bytes)); then
    printf 'download-test-fixtures: invalid byte bounds for %s: %s..%s\n' \
      "${id}" "${minimum_bytes}" "${maximum_bytes}" >&2
    return 1
  fi
  if [[ ! "${connect_timeout}" =~ ^[1-9][0-9]*$ || ! "${deadline}" =~ ^[1-9][0-9]*$ ]]; then
    printf 'download-test-fixtures: timeout values must be positive integer seconds\n' >&2
    return 1
  fi
  if (($# == 0)); then
    printf 'download-test-fixtures: no source URLs configured for %s\n' "${id}" >&2
    return 1
  fi
  local encoded_limit="${maximum_bytes}"
  if [[ "${encoding}" == "base64" ]]; then
    encoded_limit="$((((maximum_bytes + 2) / 3) * 4 + 4))"
  elif [[ "${encoding}" != "raw" ]]; then
    printf 'download-test-fixtures: unsupported encoding for %s: %s\n' "${id}" "${encoding}" >&2
    return 1
  fi

  mkdir -p "$(dirname "${destination}")"
  local candidate_url
  for candidate_url in "$@"; do
    local transfer_path
    local payload_path
    transfer_path="$(mktemp "${temporary_root}/transfer.XXXXXX")"
    payload_path="$(mktemp "${temporary_root}/payload.XXXXXX")"
    printf 'download-test-fixtures: downloading %s from %s\n' "${id}" "${candidate_url}"

    if ! "${curl_bin}" \
      --fail \
      --location \
      --show-error \
      --silent \
      --proto '=https' \
      --proto-redir '=https' \
      --connect-timeout "${connect_timeout}" \
      --max-time "${deadline}" \
      --retry-max-time "${deadline}" \
      --max-filesize "${encoded_limit}" \
      --retry 3 \
      --retry-delay 2 \
      --output "${transfer_path}" \
      "${candidate_url}"; then
      rm -f "${transfer_path}" "${payload_path}"
      printf 'download-test-fixtures: transfer failed for %s from %s\n' "${id}" "${candidate_url}" >&2
      continue
    fi

    if [[ "${encoding}" == "base64" ]]; then
      if ! fixture_decode_base64 "${transfer_path}" "${payload_path}"; then
        rm -f "${transfer_path}" "${payload_path}"
        printf 'download-test-fixtures: base64 decode failed for %s from %s\n' "${id}" "${candidate_url}" >&2
        continue
      fi
    else
      mv "${transfer_path}" "${payload_path}"
    fi
    rm -f "${transfer_path}"

    if fixture_validate "${id}" "${payload_path}" "${expected_sha256}" "${minimum_bytes}" "${maximum_bytes}"; then
      chmod 0644 "${payload_path}"
      mv "${payload_path}" "${destination}"
      return 0
    fi

    rm -f "${payload_path}"
    printf 'download-test-fixtures: integrity validation failed for %s from %s\n' "${id}" "${candidate_url}" >&2
  done

  printf 'download-test-fixtures: no valid source remained for %s\n' "${id}" >&2
  return 1
}
