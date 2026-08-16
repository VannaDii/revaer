# frozen_string_literal: true

require "open3"

module RevaerDatabaseRebaseline
  class Failure < StandardError; end

  module EnvFile
    module_function

    KEY_PATTERN = /\A[A-Z][A-Z0-9_]*\z/

    def load(path)
      values = {}
      File.readlines(path, chomp: true).each_with_index do |line, index|
        next if line.empty? || line.start_with?("#")

        key, raw_value = line.split("=", 2)
        unless key&.match?(KEY_PATTERN) && raw_value
          raise Failure, "invalid environment entry at #{path}:#{index + 1}"
        end
        raise Failure, "duplicate environment key #{key} in #{path}" if values.key?(key)

        values[key] = unquote(raw_value, path, index + 1)
      end
      values
    rescue Errno::ENOENT => error
      raise Failure, "required contract file is missing: #{error.path}"
    end

    def unquote(raw_value, path, line_number)
      return raw_value unless raw_value.start_with?("\"")

      unless raw_value.length >= 2 && raw_value.end_with?("\"")
        raise Failure, "unterminated quoted value at #{path}:#{line_number}"
      end
      raw_value[1...-1]
    end
    private_class_method :unquote
  end

  class CommandRunner
    Result = Data.define(:stdout, :stderr, :success)

    def capture(command, env: {}, stdin_data: nil, chdir: nil)
      options = {}
      options[:stdin_data] = stdin_data unless stdin_data.nil?
      options[:chdir] = chdir unless chdir.nil?
      stdout, stderr, status = Open3.capture3(env, *command, **options)
      Result.new(stdout:, stderr:, success: status.success?)
    end

    def run!(command, env: {}, stdin_data: nil, chdir: nil, redactions: [])
      result = capture(command, env:, stdin_data:, chdir:)
      return result.stdout if result.success

      detail = sanitize(result.stderr.empty? ? result.stdout : result.stderr, redactions)
      executable = File.basename(command.fetch(0))
      raise Failure, "#{executable} failed: #{detail.strip}"
    end

    private

    def sanitize(value, redactions)
      redactions.reduce(value.dup) do |sanitized, secret|
        secret.empty? ? sanitized : sanitized.gsub(secret, "[redacted]")
      end
    end
  end
end
