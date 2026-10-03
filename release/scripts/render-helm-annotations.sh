#!/usr/bin/env bash

set -euo pipefail

if [[ "$#" -ne 3 ]]; then
    echo "usage: $0 <chart-template> <annotations-file> <output-file>" >&2
    exit 1
fi

chart_template="$1"
annotations_file="$2"
output_file="$3"

if [[ ! -f "${chart_template}" ]]; then
    echo "chart template does not exist: ${chart_template}" >&2
    exit 1
fi

if [[ ! -s "${annotations_file}" ]]; then
    echo "release annotations file is missing or empty: ${annotations_file}" >&2
    exit 1
fi

awk -v annotations_file="${annotations_file}" '
    /^[[:space:]]*#[[:space:]]*__RELEASE_HELM_ANNOTATIONS__[[:space:]]*$/ {
        marker_count += 1
        while ((status = getline annotation < annotations_file) > 0) {
            print annotation
        }
        close(annotations_file)
        if (status < 0) {
            print "failed to read release annotations file" > "/dev/stderr"
            exit 1
        }
        next
    }
    { print }
    END {
        if (marker_count != 1) {
            print "expected exactly one release annotation marker" > "/dev/stderr"
            exit 1
        }
    }
' "${chart_template}" > "${output_file}"
