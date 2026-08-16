#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
work_root="$(mktemp -d "${TMPDIR:-/tmp}/revaer-media-conversion.XXXXXX")"
report_path="${MEDIA_CONVERSION_REPORT:-${repo_root}/target/media-conversion-report.md}"
trap 'rm -rf "${work_root}"' EXIT

for command in ffmpeg ffprobe jq; do
  if ! command -v "${command}" >/dev/null 2>&1; then
    printf 'Media conversion fixture requires %s\n' "${command}" >&2
    exit 1
  fi
done

source_path="${work_root}/source.mkv"
output_path="${work_root}/transcoded.mp4"
probe_path="${work_root}/probe.json"

ffmpeg -hide_banner -loglevel error -nostdin -y \
  -f lavfi -i 'testsrc2=size=160x90:rate=12:duration=1' \
  -f lavfi -i 'sine=frequency=880:sample_rate=48000:duration=1' \
  -c:v ffv1 -c:a pcm_s16le -shortest "${source_path}"
ffmpeg -hide_banner -loglevel error -nostdin -y \
  -i "${source_path}" -map 0:v:0 -map 0:a:0 \
  -c:v mpeg4 -q:v 4 -c:a aac -b:a 96k -movflags +faststart "${output_path}"
ffprobe -v error -show_format -show_streams -of json "${output_path}" > "${probe_path}"

jq -e '
  ([.streams[] | select(.codec_type == "video" and .codec_name == "mpeg4" and
    .width == 160 and .height == 90)] | length) == 1 and
  ([.streams[] | select(.codec_type == "audio" and .codec_name == "aac" and
    .sample_rate == "48000")] | length) == 1 and
  (.format.duration | tonumber) > 0
' "${probe_path}" >/dev/null

mkdir -p "$(dirname "${report_path}")"
if command -v sha256sum >/dev/null 2>&1; then
  source_sha="$(sha256sum "${source_path}" | awk '{print $1}')"
  output_sha="$(sha256sum "${output_path}" | awk '{print $1}')"
elif command -v shasum >/dev/null 2>&1; then
  source_sha="$(shasum -a 256 "${source_path}" | awk '{print $1}')"
  output_sha="$(shasum -a 256 "${output_path}" | awk '{print $1}')"
else
  echo "Media conversion fixture requires sha256sum or shasum" >&2
  exit 1
fi
duration="$(jq -r '.format.duration' "${probe_path}")"
{
  printf '# Media Conversion Fixture\n\n'
  printf -- '- Source container: Matroska (FFV1 + PCM)\n'
  printf -- '- Output container: MP4 (MPEG-4 Part 2 + AAC)\n'
  printf -- '- Output duration: %s seconds\n' "${duration}"
  printf -- '- Source SHA-256: `%s`\n' "${source_sha}"
  printf -- '- Output SHA-256: `%s`\n' "${output_sha}"
  printf -- '- Generated media cleanup: enforced by exit trap\n'
} > "${report_path}"
test -s "${report_path}"
