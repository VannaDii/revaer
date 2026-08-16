# frozen_string_literal: true

class KcovHelperError < StandardError; end

UNSAFE_PROMPT = "PS4='kcov@\${BASH_SOURCE}@\${LINENO}@'"
SAFE_PROMPT = "PS4='kcov@\${BASH_SOURCE:-kcov-inline}@\${LINENO}@'"

def prepare_helper(path)
  source = File.binread(path)
  unsafe_count = source.scan(UNSAFE_PROMPT).length
  safe_count = source.scan(SAFE_PROMPT).length

  return if unsafe_count.zero? && safe_count == 1

  unless unsafe_count == 1 && safe_count.zero?
    raise KcovHelperError,
          "kcov Bash helper does not contain exactly one recognized trace prompt"
  end

  replacement = source.sub(UNSAFE_PROMPT, SAFE_PROMPT)
  File.binwrite(path, replacement)
end

begin
  helper_path = ARGV.fetch(0)
  prepare_helper(helper_path)
rescue IndexError, SystemCallError, KcovHelperError => e
  warn "Cannot prepare kcov Bash helper: #{e.message}"
  exit 1
end
