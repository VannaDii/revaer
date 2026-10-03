#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

require_tool() {
  local tool="$1"
  if ! command -v "${tool}" >/dev/null 2>&1; then
    printf 'generate-derived-fixtures: required tool missing: %s\n' "${tool}" >&2
    exit 1
  fi
}

for tool in ffmpeg ffprobe mktemp; do
  require_tool "${tool}"
done

source_fixture="test-fixtures/source/bbb-h264.mp4"
if [[ ! -s "${source_fixture}" ]]; then
  printf 'generate-derived-fixtures: missing source fixture: %s\n' "${source_fixture}" >&2
  exit 1
fi

if ! ffprobe -v error -show_streams -of json "${source_fixture}" >/dev/null; then
  printf 'generate-derived-fixtures: source fixture is not probeable: %s\n' "${source_fixture}" >&2
  exit 1
fi

mkdir -p test-fixtures/derived

tmpdir="$(mktemp -d "${TMPDIR:-/tmp}/revaer-derived-fixtures.XXXXXX")"
cleanup_tmpdir() {
  rm -rf "${tmpdir}"
}
trap cleanup_tmpdir EXIT

write_srt_files() {
  cat >"${tmpdir}/english-full.srt" <<'SRT'
1
00:00:00,000 --> 00:00:03,000
English full subtitle line one.

2
00:00:03,000 --> 00:00:07,000
English full subtitle line two.
SRT

  cat >"${tmpdir}/english-forced.srt" <<'SRT'
1
00:00:01,000 --> 00:00:04,000
English forced subtitle.
SRT
}

move_generated() {
  local temp="$1"
  local destination="$2"

  if [[ ! -s "${temp}" ]]; then
    printf 'generate-derived-fixtures: generated empty fixture: %s\n' "${destination}" >&2
    exit 1
  fi
  mv "${temp}" "${destination}"
}

skip_existing() {
  local id="$1"
  local destination="$2"
  if [[ -s "${destination}" && "${REVAER_FIXTURE_FORCE_GENERATE:-0}" != "1" ]]; then
    printf 'generate-derived-fixtures: %s already exists: %s\n' "${id}" "${destination}"
    return 0
  fi
  return 1
}

if ! skip_existing subtitles-mkv test-fixtures/derived/subtitles.mkv; then
  write_srt_files
  temp="$(mktemp "${tmpdir}/subtitles.XXXXXX")"
  ffmpeg -hide_banner -v error -y \
    -i "${source_fixture}" \
    -f srt -i "${tmpdir}/english-full.srt" \
    -f srt -i "${tmpdir}/english-forced.srt" \
    -map 0:v:0 -map 0:a? -map 1:0 -map 2:0 \
    -c:v copy -c:a copy -c:s srt \
    -metadata:s:s:0 language=eng -metadata:s:s:0 title="English Full Subtitles" \
    -metadata:s:s:1 language=eng -metadata:s:s:1 title="English Forced Subtitles" \
    -disposition:s:0 0 -disposition:s:1 forced \
    -default_mode passthrough \
    -f matroska "${temp}"
  move_generated "${temp}" test-fixtures/derived/subtitles.mkv
fi

printf 'generate-derived-fixtures: complete\n'
