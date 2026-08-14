# frozen_string_literal: true

require "coverage"
require "fileutils"
require "json"

output_directory = ENV.fetch("REVAER_RUBY_COVERAGE_DIR")
repository_root = File.expand_path("..", __dir__)
FileUtils.mkdir_p(output_directory)

at_exit do
  result = Coverage.result
  files = result.filter_map do |path, data|
    expanded_path = File.expand_path(path)
    next unless expanded_path.start_with?("#{repository_root}/") && expanded_path.end_with?(".rb")

    branches = data.fetch(:branches).flat_map do |base, alternatives|
      alternatives.map do |alternative, hits|
        {
          line: base.fetch(2),
          key: JSON.generate([base, alternative]),
          hits:
        }
      end
    end
    {
      path: expanded_path.delete_prefix("#{repository_root}/"),
      lines: data.fetch(:lines),
      branches:
    }
  end
  payload = JSON.generate({ files: })
  output_path = File.join(output_directory, "ruby-#{Process.pid}.json")
  File.write(output_path, payload, mode: "w", encoding: "UTF-8")
rescue StandardError => e
  warn "Ruby coverage collection failed: #{e.message}"
  Process.exit!(1)
end
