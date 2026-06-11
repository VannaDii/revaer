#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

require_tool() {
  local tool="$1"
  if ! command -v "${tool}" >/dev/null 2>&1; then
    printf 'download-test-fixtures: required tool missing: %s\n' "${tool}" >&2
    exit 1
  fi
}

for tool in ffmpeg ffprobe curl git base64; do
  require_tool "${tool}"
done

mkdir -p test-fixtures/source test-fixtures/matroska test-fixtures/chromium test-fixtures/probe

download_file() {
  local id="$1"
  local url="$2"
  local destination="$3"
  shift 3
  local fallback_urls=("$@")

  if [ -s "${destination}" ] && [ "${REVAER_FIXTURE_FORCE_DOWNLOAD:-0}" != "1" ]; then
    printf 'download-test-fixtures: %s already exists: %s\n' "${id}" "${destination}"
    return 0
  fi

  local temp="${destination}.tmp.$$"
  rm -f "${temp}"
  local candidate_url
  for candidate_url in "${url}" "${fallback_urls[@]}"; do
    printf 'download-test-fixtures: downloading %s from %s\n' "${id}" "${candidate_url}"
    if curl --fail --location --show-error --retry 3 --retry-delay 2 --output "${temp}" "${candidate_url}"; then
      if [ ! -s "${temp}" ]; then
        rm -f "${temp}"
        printf 'download-test-fixtures: downloaded empty file for %s from %s\n' "${id}" "${candidate_url}" >&2
        exit 1
      fi
      mv "${temp}" "${destination}"
      return 0
    fi
    rm -f "${temp}"
    printf 'download-test-fixtures: failed to download %s from %s\n' "${id}" "${candidate_url}" >&2
  done

  printf 'download-test-fixtures: failed to download %s after trying primary and %s fallback URL(s)\n' "${id}" "${#fallback_urls[@]}" >&2
  exit 1
}

decode_base64_to_file() {
  local source_path="$1"
  local destination="$2"

  if base64 --decode <"${source_path}" >"${destination}" 2>/dev/null; then
    return 0
  fi
  if base64 -D <"${source_path}" >"${destination}" 2>/dev/null; then
    return 0
  fi
  return 1
}

download_chromium_file() {
  local id="$1"
  local source_filename="$2"
  local destination_filename="$3"
  local url="https://chromium.googlesource.com/chromium/src/+/lkgr/media/test/data/${source_filename}?format=TEXT"
  local destination="test-fixtures/chromium/${destination_filename}"

  if [ -s "${destination}" ] && [ "${REVAER_FIXTURE_FORCE_DOWNLOAD:-0}" != "1" ]; then
    printf 'download-test-fixtures: %s already exists: %s\n' "${id}" "${destination}"
    return 0
  fi

  local encoded="${destination}.b64.$$"
  local temp="${destination}.tmp.$$"
  rm -f "${encoded}" "${temp}"
  printf 'download-test-fixtures: downloading %s\n' "${id}"
  if ! curl --fail --location --show-error --retry 3 --retry-delay 2 --output "${encoded}" "${url}"; then
    rm -f "${encoded}" "${temp}"
    printf 'download-test-fixtures: failed to download %s from %s\n' "${id}" "${url}" >&2
    exit 1
  fi
  if ! decode_base64_to_file "${encoded}" "${temp}"; then
    rm -f "${encoded}" "${temp}"
    printf 'download-test-fixtures: failed to base64 decode %s from %s\n' "${id}" "${url}" >&2
    exit 1
  fi
  rm -f "${encoded}"
  if [ ! -s "${temp}" ]; then
    rm -f "${temp}"
    printf 'download-test-fixtures: decoded empty file for %s from %s\n' "${id}" "${url}" >&2
    exit 1
  fi
  mv "${temp}" "${destination}"
}

download_file \
  bbb-h264-mp4 \
  https://test-videos.co.uk/vids/bigbuckbunny/mp4/h264/360/Big_Buck_Bunny_360_10s_1MB.mp4 \
  test-fixtures/source/bbb-h264.mp4
download_file \
  bbb-h265-mp4 \
  https://test-videos.co.uk/vids/bigbuckbunny/mp4/h265/360/Big_Buck_Bunny_360_10s_1MB.mp4 \
  test-fixtures/source/bbb-h265.mp4 \
  https://web.archive.org/web/20240905220700id_/https://test-videos.co.uk/vids/bigbuckbunny/mp4/h265/360/Big_Buck_Bunny_360_10s_1MB.mp4
download_file \
  bbb-av1-mp4 \
  https://test-videos.co.uk/vids/bigbuckbunny/mp4/av1/360/Big_Buck_Bunny_360_10s_1MB.mp4 \
  test-fixtures/source/bbb-av1.mp4 \
  https://web.archive.org/web/20220722194410id_/https://test-videos.co.uk/vids/bigbuckbunny/mp4/av1/360/Big_Buck_Bunny_360_10s_1MB.mp4
download_file \
  bbb-vp8-webm \
  https://test-videos.co.uk/vids/bigbuckbunny/webm/vp8/360/Big_Buck_Bunny_360_10s_1MB.webm \
  test-fixtures/source/bbb-vp8.webm \
  https://web.archive.org/web/20201003184509id_/https://test-videos.co.uk/vids/bigbuckbunny/webm/vp8/360/Big_Buck_Bunny_360_10s_1MB.webm
download_file \
  bbb-vp9-webm \
  https://test-videos.co.uk/vids/bigbuckbunny/webm/vp9/360/Big_Buck_Bunny_360_10s_1MB.webm \
  test-fixtures/source/bbb-vp9.webm \
  https://web.archive.org/web/20220722194413id_/https://test-videos.co.uk/vids/bigbuckbunny/webm/vp9/360/Big_Buck_Bunny_360_10s_1MB.webm
download_file \
  bbb-h264-mkv \
  https://test-videos.co.uk/vids/bigbuckbunny/mkv/360/Big_Buck_Bunny_360_10s_1MB.mkv \
  test-fixtures/source/bbb-h264.mkv \
  https://web.archive.org/web/20220722194409id_/https://test-videos.co.uk/vids/bigbuckbunny/mkv/360/Big_Buck_Bunny_360_10s_1MB.mkv

matroska_tmp="$(mktemp -d "${TMPDIR:-/tmp}/revaer-matroska-test-files.XXXXXX")"
cleanup_matroska_tmp() {
  rm -rf "${matroska_tmp}"
}
trap cleanup_matroska_tmp EXIT

printf 'download-test-fixtures: cloning Matroska test corpus\n'
git clone --depth 1 https://github.com/ietf-wg-cellar/matroska-test-files.git "${matroska_tmp}/repo"

copy_matroska_file() {
  local id="$1"
  local source_name="$2"
  local destination="$3"
  local source_path="${matroska_tmp}/repo/test_files/${source_name}"

  if [ ! -s "${source_path}" ]; then
    printf 'download-test-fixtures: Matroska source missing for %s: %s\n' "${id}" "${source_path}" >&2
    exit 1
  fi
  if [ -s "${destination}" ] && [ "${REVAER_FIXTURE_FORCE_DOWNLOAD:-0}" != "1" ]; then
    printf 'download-test-fixtures: %s already exists: %s\n' "${id}" "${destination}"
    return 0
  fi
  cp "${source_path}" "${destination}"
}

copy_matroska_file mkv-basic-divx-mp3 test1.mkv test-fixtures/matroska/mkv-basic-divx-mp3.mkv
copy_matroska_file mkv-h264-aac-weird-timecode test2.mkv test-fixtures/matroska/mkv-h264-aac-weird-timecode.mkv
copy_matroska_file mkv-h264-mp3-header-stripping test3.mkv test-fixtures/matroska/mkv-h264-mp3-header-stripping.mkv
copy_matroska_file mkv-theora-vorbis-live-style test4.mkv test-fixtures/matroska/mkv-theora-vorbis-live-style.mkv
copy_matroska_file mkv-multi-audio-multi-subtitles test5.mkv test-fixtures/matroska/mkv-multi-audio-multi-subtitles.mkv
copy_matroska_file mkv-audio-gap test8.mkv test-fixtures/matroska/mkv-audio-gap.mkv

download_chromium_file chromium-bear-320x240-webm bear-320x240.webm bear-320x240.webm
download_chromium_file chromium-bear-vp9-opus-webm bear-vp9-opus.webm bear-vp9-opus.webm
download_chromium_file chromium-bear-vp8-webvtt-webm bear-vp8-webvtt.webm bear-vp8-webvtt.webm
download_chromium_file chromium-bear-1280x720-av-frag-mp4 bear-1280x720-av_frag.mp4 bear-1280x720_av_frag.mp4
download_chromium_file chromium-bear-320x180-hi10p-mp4 bear-320x180-hi10p.mp4 bear-320x180-hi10p.mp4
download_chromium_file chromium-bear-vp9-profile2-webm bear-320x240-vp9_profile2.webm bear-320x240-vp9_profile2.webm
download_chromium_file chromium-bear-v-frag-hevc-mp4 bear-320x240-v_frag-hevc.mp4 bear-320x240-v_frag-hevc.mp4
download_chromium_file chromium-bear-1280x720-aac-he-ts bear-1280x720-aac_he.ts bear-1280x720-aac_he.ts
download_chromium_file chromium-bbb-2video-2audio-mp4 bbb-320x240-2video-2audio.mp4 bbb-320x240-2video-2audio.mp4
download_chromium_file chromium-multitrack-3video-2audio-webm multitrack-3video-2audio.webm multitrack-3video-2audio.webm

printf 'download-test-fixtures: complete\n'
