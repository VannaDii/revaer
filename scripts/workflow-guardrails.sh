#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"

files=()
while IFS= read -r file; do
  files+=("${file}")
done < <(find .github/workflows .github/actions -type f \( -name '*.yml' -o -name '*.yaml' \) | sort)

if [ "${#files[@]}" -eq 0 ]; then
  exit 0
fi

failures=0

report_matches() {
  local title="$1"
  local matches="$2"

  if [ -n "${matches}" ]; then
    printf 'Workflow guardrail failed: %s\n' "${title}" >&2
    printf '%s\n' "${matches}" >&2
    printf '\n' >&2
    failures=1
  fi
}

report_missing() {
  local title="$1"
  local pattern="$2"
  local file="$3"

  if ! grep -Eq "${pattern}" "${file}"; then
    printf 'Workflow guardrail failed: %s\n' "${title}" >&2
    printf '%s: missing pattern %s\n\n' "${file}" "${pattern}" >&2
    failures=1
  fi
}

uses_matches="$(
  awk '
    {
      if ($0 !~ /^[-[:space:]]*uses:[[:space:]]*[^[:space:]#]+/) {
        next;
      }

      ref = $0;
      sub(/^[-[:space:]]*uses:[[:space:]]*/, "", ref);
      sub(/[[:space:]]*#.*/, "", ref);

      comment = "";
      if (index($0, "#") > 0) {
        comment = substr($0, index($0, "#"));
      }

      if (ref ~ /^\.\// || ref ~ /^docker:\/\//) {
        next;
      }

      if (ref !~ /@/) {
        printf "%s:%d:%s\n", FILENAME, FNR, $0;
        next;
      }

      version = ref;
      sub(/^.*@/, "", version);

      if (length(version) != 40 || version !~ /^[0-9a-f]+$/) {
        printf "%s:%d:%s\n", FILENAME, FNR, $0;
      }
    }
  ' "${files[@]}"
)"
report_matches "external GitHub actions must pin full commit SHAs instead of mutable refs or release tags" "${uses_matches}"

run_matches="$(
  awk '
    function indent_of(line,    idx, ch) {
      for (idx = 1; idx <= length(line); idx++) {
        ch = substr(line, idx, 1);
        if (ch != " ") {
          return idx - 1;
        }
      }
      return length(line);
    }

    {
      if (in_run) {
        if ($0 !~ /^[[:space:]]*$/ && indent_of($0) <= run_indent) {
          in_run = 0;
        } else {
          if ($0 ~ /\$\{\{[[:space:]]*inputs\./) {
            printf "%s:%d:%s\n", FILENAME, FNR, $0;
          }
          next;
        }
      }

      if ($0 !~ /^[[:space:]]*run:[[:space:]]*/) {
        next;
      }

      run_indent = indent_of($0);
      run_value = $0;
      sub(/^[[:space:]]*run:[[:space:]]*/, "", run_value);

      if (run_value ~ /\$\{\{[[:space:]]*inputs\./) {
        printf "%s:%d:%s\n", FILENAME, FNR, $0;
      }

      if (run_value == "" || run_value ~ /^[|>][-+]?$/) {
        in_run = 1;
      }
    }
  ' "${files[@]}"
)"
report_matches 'workflow run blocks must not interpolate ${{ inputs.* }} directly' "${run_matches}"

pr_release_skip_matches="$(
  awk '
    $0 ~ /if: github\.ref == '\''refs\/heads\/main'\'' \|\| startsWith\(github\.ref, '\''refs\/tags\/'\''\)/ {
      printf "%s:%d:%s\n", FILENAME, FNR, $0;
    }
  ' .github/workflows/pr.yml
)"
report_matches "PR Build Release must run on pull requests instead of using main/tag-only guards" "${pr_release_skip_matches}"

supply_chain_job_matches="$(
  awk '
    /^  (audit|deny|udeps):/ {
      printf "%s:%d:%s\n", FILENAME, FNR, $0;
    }
  ' .github/workflows/pr.yml
)"
report_matches "PR supply-chain checks must run in one combined job instead of standalone audit, deny, or udeps jobs" "${supply_chain_job_matches}"

report_missing "PR workflow must define the combined supply-chain job" '^  supply-chain:' .github/workflows/pr.yml
report_missing "PR supply-chain job must cache installed Cargo tool binaries" 'cargo-supply-chain-tools' .github/workflows/pr.yml
report_missing "PR supply-chain job must cache the Cargo advisory database" '~/.cargo/advisory-db' .github/workflows/pr.yml
report_missing "PR supply-chain job must install cargo-audit as a prebuilt tool" 'cargo-audit@0\.22\.0' .github/workflows/pr.yml
report_missing "PR supply-chain job must install cargo-deny as a prebuilt tool" 'cargo-deny@0\.18\.9' .github/workflows/pr.yml
report_missing "PR supply-chain job must install cargo-udeps as a prebuilt tool" 'cargo-udeps@0\.1\.57' .github/workflows/pr.yml
report_missing "PR supply-chain job must run audit through just" 'just audit' .github/workflows/pr.yml
report_missing "PR supply-chain job must run deny through just" 'just deny' .github/workflows/pr.yml
report_missing "PR supply-chain job must run udeps through just" 'just udeps' .github/workflows/pr.yml

if [ "${failures}" -ne 0 ]; then
  exit 1
fi
