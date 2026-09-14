# frozen_string_literal: true

require "json"
require "securerandom"
require_relative "native_processes"

module RevaerDatabaseRebaseline
  # Read-only debugger attachment to a backend owned by the disposable proof.
  class NativeSessions
    Capture = Data.define(:application, :debugger)
    CHILD_CALLBACKS = %w[RI_FKey_check_ins RI_FKey_check_upd].freeze
    PARENT_CALLBACKS = %w[noaction restrict cascade setnull setdefault].flat_map do |action|
      %w[upd del].map { |event| "RI_FKey_#{action}_#{event}" }
    end.freeze

    def initialize(runner:, processes:, directory:, container:, tooling:, probe:)
      raise Failure, "native target container is invalid" unless container.match?(/\Arevaer-final-proof-[a-z0-9-]+\z/)
      raise Failure, "native debugger image is not immutable" unless tooling.debugger_image.match?(/\Asha256:[a-f0-9]{64}\z/)
      raise Failure, "native observer probe is empty" unless probe.is_a?(String) && !probe.empty?

      @runner = runner
      @processes = processes
      @directory = directory
      @container = container
      @tooling = tooling
      @probe = probe
      @containers = []
    end

    def execute(command:, query:, name:, observed:, probe: @probe)
      raise Failure, "native observer probe is empty" unless probe.is_a?(String) && !probe.empty?

      psql = @processes.launch(command, name)
      header = @processes.handshake(psql, "DO $$ BEGIN NULL; END $$;\nSELECT 'native_backend:' || pg_backend_pid();\n",
                                    /\Anative_backend:[1-9][0-9]*\n\z/)
      backend = Integer(header.delete_prefix("native_backend:"), 10)
      debugger = attach(name, backend, probe) if observed
      application = @processes.finish(psql, stdin_data: "#{query}\n\\q\n")
      @processes.finish(debugger) if debugger
      stdout = application.fetch(:stdout)
      raise Failure, "native handshake framing changed" unless stdout.start_with?(header)

      result = CommandRunner::Result.new(stdout: stdout.delete_prefix(header), stderr: application.fetch(:stderr), success: true)
      Capture.new(application: result, debugger: debugger&.slice(:stdout_path, :stderr_path))
    end

    def remove_containers!
      failures = []
      @containers.each do |name|
        outcome = @runner.capture(["docker", "rm", "-fv", name])
        absent = outcome.stderr.lines.any? { |line| line.strip == "Error response from daemon: No such container: #{name}" }
        failures << name unless outcome.success || absent
      rescue StandardError => error
        failures << "#{name}: #{error.message}"
      end
      raise Failure, "native debugger cleanup failed: #{failures.join(', ')}" unless failures.empty?
    end

    def close_after_owner_cleanup!
      @processes.close_after_owner_cleanup!
    end

    private

    def write(name, bytes)
      path = File.join(@directory, name)
      File.open(path, File::WRONLY | File::CREAT | File::EXCL, 0o600) { |file| file.write(bytes) }
      path
    end

    def attach(name, backend, probe)
      debug_name = "#{@container}-observer-#{SecureRandom.hex(4)}"
      script_path = write("#{name}-debugger.gdb", script(backend, probe))
      command = ["docker", "create", "--name", debug_name, "--network", "none", "--pid", "container:#{@container}",
                 "--cap-add", "SYS_PTRACE", "--mount", "type=bind,source=#{@tooling.musl_source_directory},target=/observer-source,readonly",
                 @tooling.debugger_image, "--batch", "--nx", "--nh", "-x", "/tmp/observe.gdb"]
      write("#{name}-debugger-command.json", JSON.pretty_generate(command) + "\n")
      @containers << debug_name
      @runner.run!(command)
      @runner.run!(["docker", "cp", script_path, "#{debug_name}:/tmp/observe.gdb"])
      process = @processes.launch(["docker", "start", "--attach", debug_name], "#{name}-debugger")
      @processes.ready(process, /\AREADY:0\n\z/)
      process
    end

    def breakpoint(symbol, lines)
      ["break *#{symbol}", "commands", "silent", *lines, "continue", "end"].join("\n")
    end

    def script(backend, probe)
      root = "/proc/#{backend}/root"
      commands = ["set pagination off", "set confirm off", "set auto-load off", "set debuginfod enabled off",
                  "set may-call-functions off", "set print thread-events off", "set sysroot #{root}",
                  "file #{root}/usr/local/bin/postgres", "directory /observer-source", "attach #{backend}"]
      CHILD_CALLBACKS.each do |symbol|
        # Pinned LP64 ABI: fcinfo.context +8, TriggerData.tg_trigger +32.
        commands << breakpoint(symbol, [
          "printf \"RI:#{symbol}:%d\\n\", (int)plpgsql_variable_conflict",
          "printf \"TRIGGER:#{symbol}:%u:%u:%d\\n\", *(unsigned int*)(*(char**)$x0 + 8), *(unsigned int*)(*(char**)(*(char**)($x0 + 8) + 32)), (int)plpgsql_variable_conflict"
        ])
      end
      { "plpgsql_call_handler" => "plpgsql", "fmgr_sql" => "sql" }.each do |symbol, language|
        commands << breakpoint(symbol, ["printf \"CALL:#{language}:%u:%d\\n\", *(unsigned int*)(*(char**)$x0 + 8), (int)plpgsql_variable_conflict"])
      end
      PARENT_CALLBACKS.each do |symbol|
        commands << breakpoint(symbol, ["printf \"PARENT_RI:#{symbol}:%d\\n\", (int)plpgsql_variable_conflict"])
      end
      commands.concat([probe, 'printf "READY:%d\n", (int)plpgsql_variable_conflict', "continue"]).join("\n") + "\n"
    end
  end
end
