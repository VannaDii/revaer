# frozen_string_literal: true

require "stringio"
require "rubygems"
require_relative "../database_rebaseline/native_tooling"

module DatabaseNativeToolingTest
  Tooling = RevaerDatabaseRebaseline::NativeTooling
  Failure = RevaerDatabaseRebaseline::Failure
  BASE = "docker.io/library/postgres@sha256:#{'a' * 64}"
  BASE_ID = "sha256:#{'b' * 64}"
  CACHE = "sha256:#{'c' * 64}"
  BUILT = "sha256:#{'d' * 64}"
  MUSL = "musl-1.2.6"
  SOURCE = "test-only syscall source\n"
  SOURCE_MTIME = 1_600_000_000

  def self.metadata(packages)
    packages.map { |name, version| "P:#{name}\nV:#{version}\nA:aarch64\nC:Q1#{'A' * 27}=\n" }.join("\n") + "\n"
  end

  def self.metadata_digest(raw)
    rows = raw.split("\n\n").map do |block|
      fields = block.lines(chomp: true).to_h { |line| line.split(":", 2) }
      "#{fields.fetch('P')}=#{fields.fetch('V')} #{fields.fetch('A')} #{fields.fetch('C')}"
    end
    Digest::SHA256.hexdigest(rows.sort.join("\n") + "\n")
  end

  def self.archive(extra: nil, source: SOURCE, pax: true)
    buffer = StringIO.new("".b)
    if pax
      comment = "52 comment=#{'a' * 40}\n"
      buffer.write(Gem::Package::TarHeader.new(name: "pax_global_header", mode: 0o644,
                                             size: comment.bytesize, typeflag: "g", prefix: "").to_s)
      buffer.write(comment + "\0" * (512 - comment.bytesize))
    end
    Gem::Package::TarWriter.new(buffer) do |tar|
      [MUSL, "#{MUSL}/src", "#{MUSL}/src/thread", "#{MUSL}/src/thread/aarch64"].each { |path| tar.mkdir(path, 0o755) }
      buffer.write(Gem::Package::TarHeader.new(name: "#{MUSL}/#{Tooling::SYSCALL_SOURCE}", mode: 0o644,
        size: source.bytesize, mtime: SOURCE_MTIME, typeflag: "0", prefix: "").to_s)
      buffer.write(source + "\0" * ((512 - source.bytesize % 512) % 512))
      extra.call(tar) if extra
    end
    gzip = StringIO.new("".b)
    Zlib::GzipWriter.wrap(gzip) { |writer| writer.write(buffer.string) }
    gzip.string
  end

  class Runner
    attr_reader :commands, :containers, :dockerfile, :context_entries, :context_mode, :context_path
    attr_accessor :cache_present, :fault, :base_metadata, :observer_metadata, :archive,
                  :inspections, :hash_drift, :release, :gdb_version, :postgres_version, :missing_prefix

    def initialize(inputs, archive)
      @inputs, @archive = inputs, archive
      @commands, @containers = [], {}
      @sequence = 0
      @cache_present = true
      @missing_prefix = "Error response from daemon: No such image: "
      @base_metadata = DatabaseNativeToolingTest.metadata("libbase" => "2.0-r0", "musl" => "1.2.6-r2")
      @observer_metadata = @base_metadata + DatabaseNativeToolingTest.metadata(
        "gdb" => "16.3-r4", "libtest" => "1.0-r0", "musl-dbg" => "1.2.6-r2"
      )
      @release, @gdb_version, @postgres_version = "3.24.1", "16.3", "16.14"
      @inspections = { BASE => image(BASE_ID, base: true), CACHE => image(CACHE), BUILT => image(BUILT) }
    end

    def capture(command, **_options)
      @commands << command.dup
      raise "unexpected injected capture: #{command.inspect}" unless command.take(3) == %w[docker image inspect]

      ref = command.fetch(3)
      if @fault == :cache_inspect && ref == CACHE
        return result("", "permission denied", false)
      end
      data = ref == @tag && @built ? @inspections.fetch(BUILT) : @inspections[ref]
      data = nil if ref == CACHE && !@cache_present
      data ? result(JSON.generate([data]), "", true) : result("[]\n", "#{@missing_prefix}#{ref}\n", false)
    end

    def run!(command, **_options)
      if command.take(3) == %w[docker image inspect]
        response = capture(command)
        raise Failure, response.stderr unless response.success

        return response.stdout
      end
      @commands << command.dup
      case command.take(2)
      when %w[docker create] then create(command)
      when %w[docker start]
        fail_at!(:start)
        output(*@containers.fetch(command.last))
      when %w[docker rm]
        fail_at!(:cleanup)
        raise "removing an unowned container" unless @containers.delete(command.last)

        "removed\n"
      when %w[docker build] then build(command)
      when %w[docker image]
        raise "removing an unowned image" unless command == ["docker", "image", "rm", "--no-prune", @tag] && @built

        fail_at!(:image_cleanup)
        @built = false
        "removed\n"
      else
        raise "unexpected injected command: #{command.inspect}" unless command.first == "curl"

        File.binwrite(command.fetch(command.index("--output") + 1), @archive)
        fail_at!(:download)
        ""
      end
    end

    private

    def result(stdout, stderr, success)
      RevaerDatabaseRebaseline::CommandRunner::Result.new(stdout:, stderr:, success:)
    end

    def image(id, base: false)
      { "Id" => id, "RepoDigests" => base ? [BASE.delete_prefix("docker.io/library/")] : [],
        "Os" => "linux", "Architecture" => "arm64",
        "RootFS" => { "Type" => "layers", "Layers" => base ? [BASE_ID] : [BASE_ID, BUILT] },
        "Config" => { "User" => "root", "Entrypoint" => base ? ["docker-entrypoint.sh"] : ["gdb"] } }
    end

    def fail_at!(point)
      raise Failure, "injected #{point} failure" if Array(@fault).include?(point)
    end

    def create(command)
      fail_at!(:create_before)
      @sequence += 1
      cid = @sequence.to_s(16).rjust(64, "0")
      offset = command.index("--entrypoint")
      @containers[cid] = [command.fetch(offset + 2), command.fetch(offset + 1), command.drop(offset + 3)]
      File.write(command.fetch(command.index("--cidfile") + 1), cid)
      if @fault == :receipt_collision
        File.write(File.join(File.dirname(command.fetch(command.index("--cidfile") + 1)), "receipt.json"), "collision")
      end
      fail_at!(:create_after)
      cid
    end

    def build(command)
      @context_path = command.last
      @context_entries = Dir.children(@context_path)
      @context_mode = File.stat(@context_path).mode & 0o777
      @dockerfile = File.read(File.join(@context_path, "Dockerfile"))
      @tag = command.fetch(command.index("--tag") + 1)
      fail_at!(:build_before)
      @built = true
      File.write(command.fetch(command.index("--iidfile") + 1), BUILT)
      fail_at!(:build_after)
      "built\n"
    end

    def output(image, executable, arguments)
      case executable
      when "/bin/cat"
        return "#{@release}\n" if arguments == ["/etc/alpine-release"]

        image == BASE ? @base_metadata : @observer_metadata
      when "/usr/bin/sha256sum"
        arguments.map do |path|
          key = Tooling::FILE_INPUTS.key(path)
          hash = @hash_drift == [image, path] ? "0" * 64 : @inputs.fetch("POSTGRES_NATIVE_#{key}")
          "#{hash}  #{path}\n"
        end.join
      when "/usr/bin/gdb" then "GNU gdb (GDB) #{@gdb_version}\nfixture report\n"
      when "/usr/local/bin/postgres" then "postgres (PostgreSQL) #{@postgres_version}\n"
      else raise "unexpected probe: #{executable}"
      end
    end
  end

  class Fixture
    attr_reader :root, :inputs, :runner

    def initialize
      @root = File.realpath(Dir.mktmpdir("revaer-native-tooling-test."))
      File.write(File.join(@root, "caller-owned"), "keep\n")
      @inputs = { "DOCKERFILE_FRONTEND_IMAGE" => "docker/dockerfile:1.7@sha256:#{'e' * 64}",
                  "POSTGRES_REBASELINE_IMAGE" => BASE, "POSTGRES_REBASELINE_VERSION" => "16.14",
                  "POSTGRES_NATIVE_DEBUGGER_IMAGE" => CACHE, "POSTGRES_NATIVE_ALPINE_VERSION" => "3.24.1",
                  "POSTGRES_NATIVE_APK_PACKAGES" => "gdb=16.3-r4 libtest=1.0-r0 musl-dbg=1.2.6-r2",
                  "POSTGRES_NATIVE_MUSL_SOURCE_URL" => "https://musl.libc.org/releases/#{MUSL}.tar.gz",
                  "POSTGRES_NATIVE_MUSL_SYSCALL_CP_SHA256" => Digest::SHA256.hexdigest(SOURCE) }
      Tooling::FILE_INPUTS.each_key { |key| @inputs["POSTGRES_NATIVE_#{key}"] = Digest::SHA256.hexdigest(key) }
      @runner = Runner.new(@inputs, DatabaseNativeToolingTest.archive)
      @inputs["POSTGRES_NATIVE_APK_METADATA_SHA256"] = DatabaseNativeToolingTest.metadata_digest(@runner.observer_metadata)
      @inputs["POSTGRES_NATIVE_MUSL_SOURCE_SHA256"] = Digest::SHA256.hexdigest(@runner.archive)
    end

    def prepare(directory: @root, postgres_image: BASE)
      Tooling.new(runner: @runner, inputs: @inputs, directory:, postgres_image:).prepare!
    end

    def seed_source
      File.binwrite(File.join(@root, "#{MUSL}.tar.gz"), @runner.archive)
    end

    def close
      Find.find(@root) { |path| File.chmod(0o700, path) if File.lstat(path).directory? }
      FileUtils.remove_entry_secure(@root)
    end
  end

  class Suite
    def initialize
      @tests = 0
      @assertions = 0
    end

    def assert(condition, message)
      @assertions += 1
      raise message unless condition
    end

    def test(name)
      fixture = Fixture.new
      yield fixture
      assert(File.read(File.join(fixture.root, "caller-owned")) == "keep\n", "caller files changed")
      @tests += 1
    rescue StandardError => error
      raise "#{name}: #{error.message}", cause: error
    ensure
      fixture.close if fixture
    end

    def rejects(fixture, pattern, **options)
      error = begin
        fixture.prepare(**options)
        nil
      rescue Failure => failure
        failure
      end
      assert(error && error.message.match?(pattern), "expected #{pattern}, got #{error.inspect}")
      assert(Dir.glob(File.join(fixture.root, "native-tooling.*")).empty?, "failed setup retained its workspace")
      assert(fixture.runner.containers.empty?, "owned containers were not removed") unless Array(fixture.runner.fault).include?(:cleanup)
      error
    end

    def run
      test("qualified cached setup") do |f|
        f.seed_source
        f.inputs["POSTGRES_NATIVE_UNUSED_SECRET"] = "must-not-enter-receipt"
        prepared = f.prepare
        receipt = JSON.parse(File.read(prepared.receipt_path))
        assert(prepared.debugger_image == CACHE && prepared.frozen? && prepared.to_h.values.all?(&:frozen?), "mutable or incorrect result")
        assert(receipt.fetch("cached") && receipt.fetch("owned_image_tag").nil?, "cache ownership is wrong")
        assert(!File.read(prepared.receipt_path).include?("must-not-enter-receipt"), "unrelated input entered receipt")
        assert((File.stat(prepared.receipt_path).mode & 0o777) == 0o400, "receipt is writable")
        source_time = File.mtime(File.join(prepared.musl_source_directory, Tooling::SYSCALL_SOURCE))
        assert(source_time.to_i == SOURCE_MTIME && source_time.nsec.zero?, "verified archive source timestamp was not preserved")
        ([prepared.musl_source_directory] + Dir.glob("#{prepared.musl_source_directory}/**/*")).each do |path|
          assert((File.stat(path).mode & 0o222).zero?, "source is writable: #{path}")
        end
        assert(f.runner.commands.none? { |command| command.first == "curl" || command[1] == "build" }, "qualified cache fetched or rebuilt")
        assert(f.runner.containers.empty?, "successful probes leaked")
        f.runner.commands.select { |command| command[1] == "create" }.each do |command|
          assert(command.include?("--pull=never") && command.include?("--read-only") && command.include?("--cidfile") &&
                 command[command.index("--network") + 1] == "none", "probe is not isolated")
        end
        second = f.prepare
        assert(second.receipt_path != prepared.receipt_path, "prior receipt was reused")
      end
      test("missing cache builds pinned observer") do |f|
        f.runner.cache_present = false
        result = f.prepare
        receipt = JSON.parse(File.read(result.receipt_path))
        assert(result.debugger_image == BUILT && !receipt.fetch("cached"), "built reference is not immutable")
        assert(receipt.fetch("owned_image_tag").start_with?("revaer-native-observer:"), "build ownership is missing")
        assert(f.runner.context_entries == ["Dockerfile"] && f.runner.context_mode == 0o700, "build context is not isolated")
        assert(!File.exist?(f.runner.context_path), "build context was retained")
        assert(f.runner.dockerfile.include?("# syntax=#{f.inputs.fetch('DOCKERFILE_FRONTEND_IMAGE')}\nFROM #{BASE}\n"), "build pins are missing")
        assert(f.runner.dockerfile.include?(f.inputs.fetch("POSTGRES_NATIVE_APK_PACKAGES")), "transitive pins are missing")
        assert(f.runner.dockerfile.lines.grep(/^RUN /) == ["RUN apk add --no-cache #{f.inputs.fetch('POSTGRES_NATIVE_APK_PACKAGES')}\n"],
               "APK install differs from the complete explicit package pins")
        assert(f.runner.commands.none? { |command| command.take(3) == %w[docker image rm] }, "successful built image was removed")
        curl = f.runner.commands.find { |command| command.first == "curl" }
        assert(curl && curl[1] == "--disable" && curl.last == f.inputs.fetch("POSTGRES_NATIVE_MUSL_SOURCE_URL"), "source download is not explicit")
      end
      test("package pins sort by name, not the equals delimiter") do |f|
        f.inputs["POSTGRES_NATIVE_APK_PACKAGES"] = "gdb=16.3-r4 libtest=1.0-r0 libtest-extra=2.0-r0 musl-dbg=1.2.6-r2"
        f.runner.observer_metadata += DatabaseNativeToolingTest.metadata("libtest-extra" => "2.0-r0")
        f.inputs["POSTGRES_NATIVE_APK_METADATA_SHA256"] = DatabaseNativeToolingTest.metadata_digest(f.runner.observer_metadata)
        assert(f.prepare.debugger_image == CACHE, "name-sorted package pins were rejected")
      end
      test("lowercase Docker cache-miss diagnostic") do |f|
        f.runner.cache_present = false
        f.runner.missing_prefix = "error: no such image: "
        assert(f.prepare.debugger_image == BUILT, "current missing-image diagnostic was rejected")
      end
      test("lowercase Docker missing owned tag after failed build") do |f|
        f.runner.cache_present = false
        f.runner.missing_prefix = "error: no such image: "
        f.runner.fault = :build_before
        error = rejects(f, /build_before failure/)
        assert(!error.message.include?("cleanup"), "missing owned tag was misreported as cleanup failure")
      end
      missing_inputs
      invalid_inputs
      drift_cases
      failure_cases
      source_cases
      puts "database native tooling: #{@tests} tests, #{@assertions} assertions passed"
    end

    def missing_inputs
      fixture = Fixture.new
      keys = fixture.inputs.keys
      fixture.close
      keys.each do |key|
        test("missing #{key}") do |f|
          f.inputs.delete(key)
          rejects(f, /#{key} is missing/)
          assert(f.runner.commands.empty?, "missing input had external effects")
        end
      end
    end

    def invalid_inputs
      { "DEBUGGER_IMAGE" => "observer:latest", "GDB_SHA256" => "A" * 64,
        "MUSL_SOURCE_URL" => "https://unapproved.invalid/musl.tar.gz",
        "APK_PACKAGES" => "gdb=16.3-r4 musl-dbg",
        "ALPINE_VERSION" => "latest" }.each do |key, value|
        test("invalid #{key}") do |f|
          f.inputs["POSTGRES_NATIVE_#{key}"] = value
          rejects(f, /invalid|exact APK/)
          assert(f.runner.commands.empty?, "invalid input had external effects")
        end
      end
      test("duplicate pins") do |f|
        f.inputs["POSTGRES_NATIVE_APK_PACKAGES"] = "gdb=16.3-r4 gdb=16.3-r4 musl-dbg=1.2.6-r2"
        rejects(f, /duplicated/)
      end
      test("caller base drift") { |f| rejects(f, /PostgreSQL input drift/, postgres_image: CACHE) }
      test("symlink directory") do |f|
        link = File.join(f.root, "link")
        File.symlink(f.root, link)
        rejects(f, /without symlinks/, directory: link)
      end
    end

    def drift_cases
      {
        "cache ID" => ->(f) { f.runner.inspections[CACHE]["Id"] = BUILT },
        "cache architecture" => ->(f) { f.runner.inspections[CACHE]["Architecture"] = "amd64" },
        "cache layers" => ->(f) { f.runner.inspections[CACHE]["RootFS"]["Layers"] = [BUILT] },
        "cache entrypoint" => ->(f) { f.runner.inspections[CACHE]["Config"]["Entrypoint"] = ["sh"] },
        "base digest" => ->(f) { f.runner.inspections[BASE]["RepoDigests"] = [] },
        "base version" => ->(f) { f.runner.postgres_version = "16.13" },
        "source version" => ->(f) { f.inputs["POSTGRES_NATIVE_MUSL_SOURCE_URL"] = "https://musl.libc.org/releases/musl-1.2.5.tar.gz" },
        "extra package" => ->(f) { f.runner.observer_metadata += DatabaseNativeToolingTest.metadata("extra" => "1-r0") },
        "removed package" => ->(f) { f.runner.observer_metadata = f.runner.observer_metadata.split("\n\n").drop(1).join("\n\n") },
        "changed package" => ->(f) { f.runner.observer_metadata = f.runner.observer_metadata.sub("V:1.0-r0", "V:1.0-r1") },
        "package checksum" => ->(f) { f.runner.observer_metadata = f.runner.observer_metadata.sub("C:Q1A", "C:Q1B") },
        "duplicate package" => ->(f) { f.runner.observer_metadata += DatabaseNativeToolingTest.metadata("gdb" => "16.3-r4") },
        "malformed package" => ->(f) { f.runner.observer_metadata = "P:gdb\nV:16.3-r4\n" },
        "GDB binary" => ->(f) { f.runner.hash_drift = [CACHE, "/usr/bin/gdb"] },
        "target musl" => ->(f) { f.runner.hash_drift = [BASE, "/lib/ld-musl-aarch64.so.1"] },
        "musl debug" => ->(f) { f.runner.hash_drift = [CACHE, "/usr/lib/debug/lib/ld-musl-aarch64.so.1.debug"] },
        "Alpine release" => ->(f) { f.runner.release = "3.24.2" },
        "GDB version" => ->(f) { f.runner.gdb_version = "17.1" }
      }.each do |name, mutate|
        test("reject #{name} drift") do |f|
          mutate.call(f)
          rejects(f, /drift|unqualified|exact PostgreSQL|match the target|malformed/)
          assert(f.runner.commands.none? { |command| command[1] == "build" }, "unqualified cache was replaced")
        end
      end
    end

    def failure_cases
      %i[cache_inspect create_before create_after start cleanup download build_before build_after].each do |fault|
        test("propagate #{fault} failure") do |f|
          f.runner.fault = fault
          f.runner.cache_present = false if fault.to_s.start_with?("build")
          rejects(f, /failed|failure/)
          removed = f.runner.commands.any? { |command| command.take(3) == %w[docker image rm] }
          assert(removed == (fault == :build_after), "incorrect image cleanup")
        end
      end
      test("post-build qualification failure cleans only owned tag") do |f|
        f.runner.cache_present = false
        f.runner.hash_drift = [BUILT, "/usr/bin/gdb"]
        rejects(f, /hash drift/)
        removals = f.runner.commands.select { |command| command.take(3) == %w[docker image rm] }
        assert(removals.length == 1 && removals.first.last.start_with?("revaer-native-observer:"), "built-image cleanup is not owned")
      end
      test("unqualified built platform is cleaned") do |f|
        f.runner.cache_present = false
        f.runner.inspections[BUILT]["Architecture"] = "amd64"
        rejects(f, /unqualified/)
        assert(f.runner.commands.any? { |command| command.take(3) == %w[docker image rm] }, "unqualified built image leaked")
      end
      test("image cleanup failure is not suppressed") do |f|
        f.runner.cache_present = false
        f.runner.hash_drift = [BUILT, "/usr/bin/gdb"]
        f.runner.fault = :image_cleanup
        error = rejects(f, /image_cleanup failure/)
        assert(error.message.include?("hash drift") && error.message.include?("revaer-native-observer:"), "cleanup hid setup failure or ownership")
      end
      test("probe failure and cleanup failure are both reported") do |f|
        f.runner.fault = %i[start cleanup]
        error = rejects(f, /native probe cleanup failed/)
        assert(error.message.include?("start failure") && error.message.include?("cleanup failure"), "probe error was hidden")
      end
      test("receipt failure removes already-read-only source") do |f|
        f.runner.fault = :receipt_collision
        rejects(f, /setup failed: File exists/)
      end
    end

    def source_cases
      test("archive hash drift preserves caller archive") do |f|
        f.seed_source
        f.inputs["POSTGRES_NATIVE_MUSL_SOURCE_SHA256"] = "0" * 64
        rejects(f, /musl archive SHA-256 drift/)
        assert(File.binread(File.join(f.root, "#{MUSL}.tar.gz")) == f.runner.archive, "caller archive changed")
      end
      test("source hash drift") do |f|
        f.inputs["POSTGRES_NATIVE_MUSL_SYSCALL_CP_SHA256"] = "0" * 64
        rejects(f, /musl syscall source SHA-256 drift/)
      end
      test("source symlink") do |f|
        File.symlink(File.join(f.root, "caller-owned"), File.join(f.root, "#{MUSL}.tar.gz"))
        rejects(f, /regular file/)
      end
      {
        "traversal" => ->(tar) { tar.add_file_simple("#{MUSL}/../outside", 0o644, 0) {} },
        "symlink" => ->(tar) { tar.add_symlink("#{MUSL}/escape", "/tmp", 0o777) },
        "duplicate" => ->(tar) { tar.mkdir(MUSL, 0o755) },
        "absolute path" => ->(tar) { tar.add_file_simple("/outside", 0o644, 0) {} }
      }.each do |name, extra|
        test("reject archive #{name}") do |f|
          f.runner.archive = DatabaseNativeToolingTest.archive(extra:)
          f.inputs["POSTGRES_NATIVE_MUSL_SOURCE_SHA256"] = Digest::SHA256.hexdigest(f.runner.archive)
          rejects(f, /unsafe|duplicate/)
        end
      end
      test("invalid gzip") do |f|
        f.runner.archive = "not a gzip"
        f.inputs["POSTGRES_NATIVE_MUSL_SOURCE_SHA256"] = Digest::SHA256.hexdigest(f.runner.archive)
        rejects(f, /setup failed/)
      end
    end
  end
end

DatabaseNativeToolingTest::Suite.new.run
