# frozen_string_literal: true

require_relative "support"

module RevaerDatabaseRebaseline
  # The proof owner removes its containers before closing these attached clients.
  class NativeProcesses
    READY_BYTES = 1_048_576

    def initialize(directory:, readiness_seconds: 30, completion_seconds: 150, cleanup_seconds: 15)
      stat = File.lstat(directory)
      raise Failure, "native evidence directory must be private and owned" unless stat.directory? && stat.uid == Process.uid && (stat.mode & 0o077).zero?
      limits = [readiness_seconds, completion_seconds, cleanup_seconds]
      raise Failure, "native process deadlines must be positive and finite" unless limits.all? { |value| value.is_a?(Numeric) && value.positive? && value.finite? }

      @directory = File.realpath(directory)
      @readiness_seconds = readiness_seconds
      @completion_seconds = completion_seconds
      @cleanup_seconds = cleanup_seconds
      @processes = []
    rescue SystemCallError => error
      raise Failure, "native evidence directory unavailable: #{error.message}"
    end

    def launch(command, name)
      raise Failure, "native process label is invalid" unless name.is_a?(String) && name.match?(/\A[a-z0-9][a-z0-9-]*\z/)
      raise Failure, "native process command is invalid" unless command.is_a?(Array) && !command.empty? && command.all? { |value| value.is_a?(String) && !value.empty? && !value.include?("\0") }

      stdout_path = File.join(@directory, "#{name}.stdout")
      stderr_path = File.join(@directory, "#{name}.stderr")
      output_file = create_file(stdout_path)
      begin
        error_file = create_file(stderr_path)
        input, output, error, waiter = Open3.popen3(*command)
      rescue StandardError
        output_file.close
        error_file&.close
        raise
      end
      record = { input:, output:, error:, waiter:, stdout_path:, stderr_path:,
                 stdout_file: output_file, stderr_file: error_file, ready: false }
      @processes << record
      record[:error_reader] = reader(error, error_file)
      record
    rescue SystemCallError => error
      raise Failure, "native process launch failed: #{error.message}"
    end

    def handshake(process, bytes, pattern)
      owned!(process)
      raise Failure, "native handshake must be bounded and unique" unless bytes.is_a?(String) && bytes.bytesize <= 1024 && !process.fetch(:ready)
      raise Failure, "native readiness pattern is invalid" unless pattern.is_a?(Regexp)

      process.fetch(:input).write(bytes)
      process.fetch(:input).flush
      ready(process, pattern)
    rescue SystemCallError, IOError => error
      raise Failure, "native handshake transport failed: #{error.message}"
    end

    def ready(process, pattern)
      owned!(process)
      raise Failure, "native readiness already consumed" if process.fetch(:ready)
      raise Failure, "native readiness pattern is invalid" unless pattern.is_a?(Regexp)

      deadline = monotonic + @readiness_seconds
      pending = +"".b
      total = 0
      loop do
        remaining = deadline - monotonic
        output = process.fetch(:output)
        raise Failure, "native process readiness timed out" unless remaining.positive? && IO.select([output], nil, nil, remaining)

        bytes = output.read_nonblock(16_384, exception: false)
        next if bytes == :wait_readable
        raise Failure, "native process exited before readiness" if bytes.nil?

        process.fetch(:stdout_file).write(bytes)
        process.fetch(:stdout_file).flush
        total += bytes.bytesize
        raise Failure, "native readiness prelude exceeds bound" if total > READY_BYTES

        pending << bytes
        while (newline = pending.index("\n"))
          line = pending.slice!(0, newline + 1)
          next unless line.match?(pattern)

          process[:ready] = true
          process[:output_reader] = reader(output, process.fetch(:stdout_file))
          return line
        end
      end
    rescue SystemCallError, IOError => error
      raise Failure, "native readiness transport failed: #{error.message}"
    end

    def finish(process, stdin_data: nil)
      owned!(process)
      raise Failure, "native process readiness was not established" unless process.fetch(:ready)
      raise Failure, "native process already completed" if process[:finished]
      raise Failure, "native process completion already started" if process[:finishing]
      raise Failure, "native stdin payload must be a string" unless stdin_data.nil? || stdin_data.is_a?(String)

      input = process.fetch(:input)
      raise Failure, "native process input closed before payload" if stdin_data && input.closed?
      process[:finishing] = true
      process[:writer] = managed_thread do
        input.write(stdin_data) if stdin_data
        input.close unless input.closed?
      end
      raise Failure, "native process did not finish" unless process.fetch(:waiter).join(@completion_seconds)

      join_threads!(process)
      close_files!(process)
      process[:finished] = true
      raise Failure, "native process returned failure" unless process.fetch(:waiter).value.success?

      { stdout: File.binread(process.fetch(:stdout_path)), stderr: File.binread(process.fetch(:stderr_path)) }
    rescue SystemCallError, IOError => error
      raise Failure, "native completion transport failed: #{error.message}"
    end

    def close_after_owner_cleanup!
      failures = []
      @processes.each do |process|
        begin
          process.fetch(:input).close unless process.fetch(:input).closed?
          raise Failure, "native owned client was not reaped" unless process.fetch(:waiter).join(@cleanup_seconds)

          process[:output_reader] ||= reader(process.fetch(:output), process.fetch(:stdout_file))
          join_threads!(process)
        rescue StandardError => error
          failures << error.message
        ensure
          close_files!(process)
          %i[output error].each do |key|
            process.fetch(key).close unless process.fetch(key).closed?
          end
        end
      end
      raise Failure, "native client cleanup failed: #{failures.join('; ')}" unless failures.empty?
    end

    private

    def owned!(process)
      raise Failure, "native process is not owned by this capture" unless @processes.any? { |record| record.equal?(process) }
    end

    def create_file(path)
      File.open(path, File::WRONLY | File::CREAT | File::EXCL, 0o600)
    end

    def managed_thread(&block)
      Thread.new do
        Thread.current.report_on_exception = false
        block.call
      end
    end

    def reader(input, output)
      managed_thread { IO.copy_stream(input, output) }
    end

    def join_threads!(process)
      failures = []
      %i[writer output_reader error_reader].each do |key|
        thread = process[key]
        next unless thread

        begin
          raise Failure, "native #{key} did not finish" unless thread.join(@cleanup_seconds)
          thread.value
        rescue StandardError => error
          failures << "#{key}: #{error.message}"
        end
      end
      raise Failure, "native stream failed: #{failures.join('; ')}" unless failures.empty?
    end

    def close_files!(process)
      %i[stdout_file stderr_file].each do |key|
        file = process.fetch(key)
        file.close unless file.closed?
      end
    end

    def monotonic
      Process.clock_gettime(Process::CLOCK_MONOTONIC)
    end
  end
end
