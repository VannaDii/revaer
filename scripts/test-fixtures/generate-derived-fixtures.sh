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

for tool in ffmpeg ffprobe curl git base64; do
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

duration="$(
  ffprobe -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 "${source_fixture}" \
    | awk 'NR == 1 && $1 + 0 > 0 { printf "%.3f", $1 + 0 }'
)"
if [[ -z "${duration}" ]]; then
  printf 'generate-derived-fixtures: unable to derive duration from %s\n' "${source_fixture}" >&2
  exit 1
fi

mkdir -p test-fixtures/derived
readonly tone_440="sine=frequency=440:sample_rate=48000"

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

if ! skip_existing multi-audio-mkv test-fixtures/derived/multi-audio.mkv; then
  temp="test-fixtures/derived/multi-audio.tmp.$$.mkv"
  rm -f "${temp}"
  ffmpeg -hide_banner -v error -y \
    -i "${source_fixture}" \
    -f lavfi -t "${duration}" -i "${tone_440}" \
    -f lavfi -t "${duration}" -i "sine=frequency=554:sample_rate=48000" \
    -f lavfi -t "${duration}" -i "sine=frequency=659:sample_rate=48000" \
    -f lavfi -t "${duration}" -i "anullsrc=channel_layout=stereo:sample_rate=48000" \
    -map 0:v:0 -map 1:a:0 -map 2:a:0 -map 3:a:0 -map 4:a:0 \
    -c:v copy -c:a:0 aac -c:a:1 libopus -c:a:2 ac3 -c:a:3 aac \
    -metadata:s:a:0 language=eng -metadata:s:a:0 title="English AAC Tone" \
    -metadata:s:a:1 language=jpn -metadata:s:a:1 title="Japanese Opus Tone" \
    -metadata:s:a:2 language=spa -metadata:s:a:2 title="Spanish AC3 Tone" \
    -metadata:s:a:3 language=und -metadata:s:a:3 title="Undeclared Silent AAC" \
    -shortest "${temp}"
  move_generated "${temp}" test-fixtures/derived/multi-audio.mkv
fi

if ! skip_existing subtitles-mkv test-fixtures/derived/subtitles.mkv; then
  write_srt_files
  temp="test-fixtures/derived/subtitles.tmp.$$.mkv"
  rm -f "${temp}"
  ffmpeg -hide_banner -v error -y \
    -i "${source_fixture}" \
    -f srt -i "${tmpdir}/english-full.srt" \
    -f srt -i "${tmpdir}/english-forced.srt" \
    -map 0:v:0 -map 0:a? -map 1:0 -map 2:0 \
    -c:v copy -c:a copy -c:s srt \
    -metadata:s:s:0 language=eng -metadata:s:s:0 title="English Full Subtitles" \
    -metadata:s:s:1 language=eng -metadata:s:s:1 title="English Forced Subtitles" \
    -disposition:s:0 0 -disposition:s:1 forced \
    "${temp}"
  move_generated "${temp}" test-fixtures/derived/subtitles.mkv
fi

if ! skip_existing video-only-mp4 test-fixtures/derived/video-only.mp4; then
  temp="test-fixtures/derived/video-only.tmp.$$.mp4"
  rm -f "${temp}"
  ffmpeg -hide_banner -v error -y -i "${source_fixture}" -map 0:v:0 -c:v copy -an -sn "${temp}"
  move_generated "${temp}" test-fixtures/derived/video-only.mp4
fi

if ! skip_existing audio-only-m4a test-fixtures/derived/audio-only.m4a; then
  temp="test-fixtures/derived/audio-only.tmp.$$.m4a"
  rm -f "${temp}"
  ffmpeg -hide_banner -v error -y \
    -f lavfi -t "${duration}" -i "${tone_440}" \
    -vn -c:a aac -movflags +faststart "${temp}"
  move_generated "${temp}" test-fixtures/derived/audio-only.m4a
fi

if ! skip_existing silent-audio-mp4 test-fixtures/derived/silent-audio.mp4; then
  temp="test-fixtures/derived/silent-audio.tmp.$$.mp4"
  rm -f "${temp}"
  ffmpeg -hide_banner -v error -y \
    -i "${source_fixture}" \
    -f lavfi -t "${duration}" -i "anullsrc=channel_layout=stereo:sample_rate=48000" \
    -map 0:v:0 -map 1:a:0 -c:v copy -c:a aac -shortest -movflags +faststart "${temp}"
  move_generated "${temp}" test-fixtures/derived/silent-audio.mp4
fi

if ! skip_existing h264-aac-ts test-fixtures/derived/h264-aac.ts; then
  temp="test-fixtures/derived/h264-aac.tmp.$$.ts"
  rm -f "${temp}"
  ffmpeg -hide_banner -v error -y \
    -i "${source_fixture}" \
    -f lavfi -t "${duration}" -i "${tone_440}" \
    -map 0:v:0 -map 1:a:0 -c:v copy -c:a aac -f mpegts "${temp}"
  move_generated "${temp}" test-fixtures/derived/h264-aac.ts
fi

if ! skip_existing h264-aac-mov test-fixtures/derived/h264-aac.mov; then
  temp="test-fixtures/derived/h264-aac.tmp.$$.mov"
  rm -f "${temp}"
  ffmpeg -hide_banner -v error -y \
    -i "${source_fixture}" \
    -f lavfi -t "${duration}" -i "${tone_440}" \
    -map 0:v:0 -map 1:a:0 -c:v copy -c:a aac "${temp}"
  move_generated "${temp}" test-fixtures/derived/h264-aac.mov
fi

if ! skip_existing mpeg4-mp3-avi test-fixtures/derived/mpeg4-mp3.avi; then
  temp="test-fixtures/derived/mpeg4-mp3.tmp.$$.avi"
  rm -f "${temp}"
  ffmpeg -hide_banner -v error -y \
    -i "${source_fixture}" \
    -f lavfi -t "${duration}" -i "${tone_440}" \
    -map 0:v:0 -map 1:a:0 -c:v mpeg4 -q:v 5 -c:a libmp3lame -q:a 4 "${temp}"
  move_generated "${temp}" test-fixtures/derived/mpeg4-mp3.avi
fi

printf 'generate-derived-fixtures: complete\n'
