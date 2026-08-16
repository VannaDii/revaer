# frozen_string_literal: true

require "json"

results_directory = File.expand_path(
  ENV.fetch("E2E_COVERAGE_DIR", "tests/test-results"),
  Dir.pwd
)

%w[1 2 3].each do |shard|
  %w[api ui].each do |kind|
    pattern = File.join(results_directory, "#{kind}-coverage-*-shard-#{shard}*.json")
    files = Dir.glob(pattern).sort
    abort("Shard #{shard} did not provide #{kind.upcase} coverage records") if files.empty?

    covered = files.flat_map do |path|
      parsed = JSON.parse(File.read(path, encoding: "UTF-8"))
      abort("#{path} must contain a JSON array") unless parsed.is_a?(Array)

      parsed
    end
    abort("Shard #{shard} provided empty #{kind.upcase} coverage records") if covered.empty?
  rescue JSON::ParserError => e
    abort("Shard #{shard} #{kind.upcase} coverage is invalid JSON: #{e.message}")
  end
end

puts "UI E2E coverage records verified for shards 1, 2, and 3"
