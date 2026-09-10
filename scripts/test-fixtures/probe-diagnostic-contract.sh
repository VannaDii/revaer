#!/usr/bin/env bash

# ADR 578 F1 expires on any identity, snapshot, diagnostic or tool-profile drift.
fixture_probe_policy_validate() {
  jq -e '
    (.fixtures | type == "array" and length > 0) and
    all(.fixtures[];
      (.id | type == "string" and test("^[a-z0-9][a-z0-9-]*$")) and
      (.path | type == "string" and length > 0) and
      ((has("allowProbeDiagnostics") | not) or
        (.allowProbeDiagnostics | type == "boolean")) and
      (.id != "mkv-theora-vorbis-live-style" or (has("allowProbeDiagnostics") | not)) and
      ((has("probeDiagnosticContract") | not) or
        (.probeDiagnosticContract == "adr578-f1" and
         .id == "mkv-theora-vorbis-live-style" and
         (has("allowProbeDiagnostics") | not)))) and
    ([.fixtures[].id] | length == (unique | length))
  ' "$1" >/dev/null
}

fixture_f1_identity_validate() {
  local manifest_path="$1" lock_path="$2"
  jq -s -e '
    "e6965e5ca666322ed93e2748a10a4f132309e005" as $revision |
    "mkv-theora-vorbis-live-style" as $id |
    "test-fixtures/matroska/mkv-theora-vorbis-live-style.mkv" as $path |
    (.[1].upstreams.matroska.revision == $revision) and
    (.[1].upstreams.matroska.repository == "https://github.com/ietf-wg-cellar/matroska-test-files") and
    ([.[0].fixtures[] | select(.id == $id) |
      {id, path, source, generated, shouldDownload, shouldGenerate, probeDiagnosticContract}] ==
      [{id: $id, path: $path,
        source: ("https://github.com/ietf-wg-cellar/matroska-test-files/blob/" + $revision + "/test_files/test4.mkv"),
        generated: false, shouldDownload: true, shouldGenerate: false,
        probeDiagnosticContract: "adr578-f1"}]) and
    ([.[1].sources[] | select(.id == $id or .path == $path)] ==
      [{id: $id, path: $path, encoding: "raw",
        sha256: "43df750a2a01a37949791b717051b41522081a266b71d113be4b713063843699",
        minimumBytes: 21313902, maximumBytes: 21313902,
        urls: [("https://raw.githubusercontent.com/ietf-wg-cellar/matroska-test-files/" + $revision + "/test_files/test4.mkv")]}])
  ' "${manifest_path}" "${lock_path}" >/dev/null
}

fixture_f1_snapshot_validate() {
  local snapshot="$1" digest
  [[ -f "${snapshot}" && ! -L "${snapshot}" ]] || return 1
  digest="$(fixture_sha256 "${snapshot}")" || return 1
  [[ "${digest}" == "7d98717b7c956c000ce4cdfdc6da9f84aebd632db51c5e8f6c4a635b2f797d25" ]]
}

fixture_f1_diagnostic_validate() {
  local diagnostic_path="$1" diagnostic_bytes diagnostic_line
  local LC_ALL=C
  diagnostic_bytes="$(fixture_size "${diagnostic_path}")" || return 1
  ((diagnostic_bytes > 0 && diagnostic_bytes <= 131)) || return 1
  IFS= read -r diagnostic_line <"${diagnostic_path}" || return 1
  local pattern="^\\[matroska,webm @ 0x[0-9a-f]{1,16}\\] Length 5 indicated by an EBML number's first byte 0x0a at pos 35 \\(0x23\\) exceeds max length 4\\.$"
  [[ "${diagnostic_line}" =~ ${pattern} ]] || return 1
  # read can discard NUL bytes; byte comparison also rejects extra lines/bytes.
  cmp -s "${diagnostic_path}" <(printf '%s\n' "${diagnostic_line}")
}

fixture_f1_tool_profile() {
  local digest
  digest="$(fixture_sha256 "$1")" || return 1
  case "${digest}" in
    8d4ba0f1aaef40839cba09f51dbcc0efbb58d1c65dc490385e46115bc8e0217c)
      printf 'historical-alpine-8.0.1-r1\n' ;;
    abd50a4468578ece7323bce6d220a2c909c5a38ce7328929947dda86a8bfab41)
      printf 'homebrew-9.0.1\n' ;;
    *) return 1 ;;
  esac
}

fixture_f1_retain_evidence() {
  local diagnostic_path="$1" version_path="$2" evidence_parent="$3" profile="$4"
  local evidence_directory diagnostic_digest version_digest
  mkdir -p "${evidence_parent}" || return 1
  evidence_directory="$(mktemp -d "${evidence_parent}/adr578-f1.XXXXXX")" || return 1
  install -m 0600 "${diagnostic_path}" "${evidence_directory}/probe.stderr" || return 1
  install -m 0600 "${version_path}" "${evidence_directory}/ffprobe-version.txt" || return 1
  [[ -s "${evidence_directory}/probe.stderr" && -s "${evidence_directory}/ffprobe-version.txt" ]] || return 1
  cmp -s "${diagnostic_path}" "${evidence_directory}/probe.stderr" || return 1
  cmp -s "${version_path}" "${evidence_directory}/ffprobe-version.txt" || return 1
  diagnostic_digest="$(fixture_sha256 "${diagnostic_path}")" || return 1
  version_digest="$(fixture_sha256 "${version_path}")" || return 1
  jq -n --arg profile "${profile}" --arg diagnostic_sha256 "${diagnostic_digest}" \
    --arg version_report_sha256 "${version_digest}" '{
      contract: "adr578-f1", fixture: "mkv-theora-vorbis-live-style",
      classification: "exact-locked-fixture-recovery-diagnostic", count: 1,
      source_sha256: "43df750a2a01a37949791b717051b41522081a266b71d113be4b713063843699",
      snapshot_sha256: "7d98717b7c956c000ce4cdfdc6da9f84aebd632db51c5e8f6c4a635b2f797d25",
      profile: $profile, version_report_sha256: $version_report_sha256,
      diagnostic_sha256: $diagnostic_sha256,
      diagnostic_file: "probe.stderr", version_report_file: "ffprobe-version.txt"
    }' >"${evidence_directory}/classification.json" || return 1
  [[ -s "${evidence_directory}/classification.json" ]] || return 1
  printf 'probe-fixtures: ADR578 F1 exact-locked-fixture-recovery-diagnostic; fixture=mkv-theora-vorbis-live-style; count=1; profile=%s; evidence=%s\n' \
    "${profile}" "${evidence_directory}"
}
