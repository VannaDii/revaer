# frozen_string_literal: true

require "digest"
require "fileutils"
require "find"
require "json"
require "rubygems/package"
require "stringio"
require "tmpdir"
require "zlib"
require_relative "support"

module RevaerDatabaseRebaseline
  # Test preparation only. The caller owns successful outputs and any built image.
  # A caller-provided musl archive in directory is verified, never modified.
  class NativeTooling
    Prepared = Data.define(:debugger_image, :musl_source_directory, :receipt_path)
    PLATFORM = "linux/arm64"
    SHA256 = /\A[0-9a-f]{64}\z/
    IMAGE_ID = /\Asha256:[0-9a-f]{64}\z/
    PACKAGE = /\A[a-z0-9][a-z0-9+_.-]*=\d[a-zA-Z0-9._+~]*-r\d+\z/
    SYSCALL_SOURCE = "src/thread/aarch64/syscall_cp.s"
    FILE_INPUTS = {
      "GDB_SHA256" => "/usr/bin/gdb",
      "MUSL_SHA256" => "/lib/ld-musl-aarch64.so.1",
      "MUSL_DEBUG_SHA256" => "/usr/lib/debug/lib/ld-musl-aarch64.so.1.debug",
      "POSTGRES_SHA256" => "/usr/local/bin/postgres"
    }.freeze
    INPUT_PATTERNS = {
      "DEBUGGER_IMAGE" => IMAGE_ID,
      "ALPINE_VERSION" => /\A\d+\.\d+\.\d+\z/,
      "APK_METADATA_SHA256" => SHA256,
      "MUSL_SOURCE_URL" => %r{\Ahttps://musl\.libc\.org/releases/musl-\d+\.\d+\.\d+\.tar\.gz\z},
      "MUSL_SOURCE_SHA256" => SHA256,
      "MUSL_SYSCALL_CP_SHA256" => SHA256
    }.merge(FILE_INPUTS.to_h { |key, _path| [key, SHA256] }).freeze

    def initialize(runner:, inputs:, directory:, postgres_image:)
      @runner = runner
      @inputs = inputs.transform_values { |value| value.is_a?(String) ? value.dup.freeze : value }.freeze
      @directory = File.expand_path(directory)
      @postgres_image = postgres_image.dup.freeze
    end

    def prepare!
      validate_inputs!
      workspace = Dir.mktmpdir("native-tooling.", @directory)
      @workspace = workspace
      completed = false
      begin
        base = inspect_image!(@postgres_image)
        verify_base!(base)
        base_packages = inventory!(@postgres_image)
        verify_source_version!(base_packages)
        cached = cached_image
        image = cached ? cached.fetch("Id") : build_image!
        observer = cached || inspect_image!(image)
        qualify!(observer, image, base, base_packages)
        source = prepare_source!
        receipt = write_receipt!(image, base, source, cached: !cached.nil?)
        result = Prepared.new(debugger_image: image.freeze, musl_source_directory: source.freeze,
                              receipt_path: receipt.freeze)
        completed = true
        result
      ensure
        unless completed
          setup_error = $!
          begin
            cleanup_failed_image!
          rescue Failure => cleanup_error
            detail = setup_error ? "#{setup_error.message}; " : ""
            raise Failure, "#{detail}#{cleanup_error.message}"
          ensure
            remove_workspace!(workspace)
          end
        end
      end
    rescue SystemCallError, IOError, JSON::ParserError, Zlib::Error, Gem::Package::TarInvalidError => error
      raise Failure, "native tooling setup failed: #{error.message}"
    end

    private

    def required(key, pattern)
      value = @inputs[key]
      raise Failure, "#{key} is missing" unless value.is_a?(String) && !value.empty?
      raise Failure, "#{key} is invalid" unless value.match?(pattern)

      value
    end

    def input(suffix)
      @inputs.fetch("POSTGRES_NATIVE_#{suffix}")
    end

    def validate_inputs!
      INPUT_PATTERNS.each { |suffix, pattern| required("POSTGRES_NATIVE_#{suffix}", pattern) }
      required("DOCKERFILE_FRONTEND_IMAGE", %r{\Adocker/dockerfile:\d+\.\d+(?:\.\d+)?@sha256:[0-9a-f]{64}\z})
      required("POSTGRES_REBASELINE_VERSION", /\A\d+\.\d+\z/)
      base = required("POSTGRES_REBASELINE_IMAGE", %r{\Adocker\.io/library/postgres@sha256:[0-9a-f]{64}\z})
      raise Failure, "native tooling PostgreSQL input drift" unless @postgres_image == base

      packages = required("POSTGRES_NATIVE_APK_PACKAGES", /\A\S+(?: \S+)*\z/).split(" ")
      unless packages.all? { |package| package.match?(PACKAGE) } && packages == packages.sort_by { |package| package.split("=", 2).first }
        raise Failure, "POSTGRES_NATIVE_APK_PACKAGES must contain sorted exact APK versions"
      end
      @packages = packages.to_h { |package| package.split("=", 2) }
      unless @packages.length == packages.length && %w[gdb musl-dbg].all? { |name| @packages.key?(name) }
        raise Failure, "native APK pins are duplicated or missing gdb/musl-dbg"
      end
      unless File.directory?(@directory) && File.realpath(@directory) == @directory
        raise Failure, "native tooling directory must be an existing directory without symlinks"
      end
      @owned_tag = nil
    end

    def image_record!(raw)
      entries = JSON.parse(raw)
      unless entries.is_a?(Array) && entries.length == 1 && entries.first.is_a?(Hash)
        raise Failure, "native image inspection is not a single image"
      end
      image = entries.first
      unless image["Id"].is_a?(String) && image["Id"].match?(IMAGE_ID)
        raise Failure, "native image ID is unqualified"
      end
      image
    end

    def parse_image!(raw)
      image = image_record!(raw)
      unless image["RootFS"].is_a?(Hash) && image["Config"].is_a?(Hash)
        raise Failure, "native image configuration is malformed"
      end
      layers = image.dig("RootFS", "Layers")
      unless image["Os"] == "linux" && image["Architecture"] == "arm64" && image.dig("RootFS", "Type") == "layers" &&
             layers.is_a?(Array) && !layers.empty? && layers.all? { |layer| layer.is_a?(String) && layer.match?(IMAGE_ID) }
        raise Failure, "native image identity, platform or layers are unqualified"
      end
      image
    end

    def inspect_image!(reference)
      image = parse_image!(@runner.run!(["docker", "image", "inspect", reference]))
      if reference.match?(IMAGE_ID) && image.fetch("Id") != reference
        raise Failure, "native image ID drift"
      end
      image
    end

    def verify_base!(image)
      digests = image["RepoDigests"]
      unless digests.is_a?(Array) && (digests & [@postgres_image, @postgres_image.delete_prefix("docker.io/library/")]).any?
        raise Failure, "PostgreSQL base digest drift"
      end
      hashes!(@postgres_image, FILE_INPUTS.slice("MUSL_SHA256", "POSTGRES_SHA256"))
      version = probe!(@postgres_image, "/usr/local/bin/postgres", "--version").strip
      unless version == "postgres (PostgreSQL) #{@inputs.fetch('POSTGRES_REBASELINE_VERSION')}"
        raise Failure, "PostgreSQL base version drift"
      end
    end

    def cached_image
      reference = input("DEBUGGER_IMAGE")
      result = @runner.capture(["docker", "image", "inspect", reference])
      if result.success
        image = parse_image!(result.stdout)
        raise Failure, "cached native image ID drift" unless image.fetch("Id") == reference

        return image
      end
      return nil if missing_image?(result.stderr, reference)

      raise Failure, "native image cache inspection failed: #{result.stderr.strip}"
    end

    def missing_image?(stderr, reference)
      stderr.match?(/\A(?:(?:Error response from daemon|[Ee]rror): )?[Nn]o such image: #{Regexp.escape(reference)}\s*\z/)
    end

    def probe!(image, executable, *arguments)
      cidfile = File.join(@workspace, "probe.cid")
      begin
        @runner.run!(["docker", "create", "--pull=never", "--platform", PLATFORM, "--network", "none",
                      "--read-only", "--cap-drop", "ALL", "--security-opt", "no-new-privileges",
                      "--cidfile", cidfile, "--entrypoint", executable, image, *arguments])
        @runner.run!(["docker", "start", "--attach", identifier!(cidfile, /\A[0-9a-f]{64}\z/)])
      ensure
        if File.exist?(cidfile) || File.symlink?(cidfile)
          probe_error = $!
          cid = identifier!(cidfile, /\A[0-9a-f]{64}\z/)
          begin
            @runner.run!(["docker", "rm", "--force", "--volumes", cid])
          rescue Failure => cleanup_error
            detail = probe_error ? "#{probe_error.message}; " : ""
            raise Failure, "#{detail}native probe cleanup failed for #{cid}: #{cleanup_error.message}"
          end
          File.unlink(cidfile)
        end
      end
    end

    def identifier!(path, pattern)
      unless File.lstat(path).file? && (value = File.read(path).strip).match?(pattern)
        raise Failure, "native setup identifier is invalid: #{File.basename(path)}"
      end
      value
    end

    def inventory!(image)
      raw = probe!(image, "/bin/cat", "/lib/apk/db/installed")
      packages = {}
      raw.split("\n\n").each do |block|
        fields = block.lines(chomp: true).grep(/\A[PCVA]:/).map { |line| line.split(":", 2) }
        package = fields.to_h
        unless fields.length == 4 && package.keys.sort == %w[A C P V] &&
               package.fetch("P").match?(/\A[.a-z0-9][a-z0-9+_.-]*\z/) &&
               package.fetch("V").match?(/\A\d[a-zA-Z0-9._+~-]*\z/) &&
               %w[aarch64 noarch].include?(package.fetch("A")) &&
               package.fetch("C").match?(/\AQ1[A-Za-z0-9+\/]{27}=\z/) && !packages.key?(package.fetch("P"))
          raise Failure, "native APK metadata is malformed or duplicated"
        end
        packages[package.fetch("P")] = package
      end
      raise Failure, "native APK metadata is empty" if packages.empty?

      packages
    end

    def verify_source_version!(packages)
      version = File.basename(input("MUSL_SOURCE_URL"), ".tar.gz").delete_prefix("musl-")
      unless packages.dig("musl", "V") == @packages.fetch("musl-dbg") &&
             @packages.fetch("musl-dbg").sub(/-r\d+\z/, "") == version
        raise Failure, "musl source/debug package does not match the target musl version"
      end
    end

    def build_image!
      context = Dir.mktmpdir("build.", @workspace)
      iidfile = File.join(@workspace, "observer.iid")
      @owned_tag = "revaer-native-observer:#{File.basename(@workspace).delete_prefix('native-tooling.')}"
      dockerfile = <<~DOCKERFILE
        # syntax=#{@inputs.fetch('DOCKERFILE_FRONTEND_IMAGE')}
        FROM #{@postgres_image}
        USER root
        RUN apk add --no-cache #{@packages.map { |name, version| "#{name}=#{version}" }.join(' ')}
        ENTRYPOINT ["gdb"]
      DOCKERFILE
      File.write(File.join(context, "Dockerfile"), dockerfile, mode: "wx", perm: 0o600)
      @runner.run!(["docker", "build", "--pull=false", "--no-cache", "--platform", PLATFORM,
                    "--iidfile", iidfile, "--tag", @owned_tag, context])
      identifier!(iidfile, IMAGE_ID)
    ensure
      FileUtils.remove_entry_secure(context) if context
    end

    def qualify!(observer, image, base, base_packages)
      layers = observer.fetch("RootFS").fetch("Layers")
      base_layers = base.fetch("RootFS").fetch("Layers")
      unless layers.length == base_layers.length + 1 && layers.take(base_layers.length) == base_layers &&
             observer.dig("Config", "Entrypoint") == ["gdb"] && observer.dig("Config", "User") == "root"
        raise Failure, "native observer does not extend the exact PostgreSQL base"
      end
      packages = inventory!(image)
      desired = base_packages.transform_values { |package| package.fetch("V") }.merge(@packages)
      delta = packages.reject { |name, package| base_packages[name] == package }
      unless packages.transform_values { |package| package.fetch("V") } == desired && delta.keys.sort == @packages.keys
        raise Failure, "native APK added/changed package set drift"
      end
      rows = packages.values.map { |package| "#{package.fetch('P')}=#{package.fetch('V')} #{package.fetch('A')} #{package.fetch('C')}" }.sort
      verify_digest!(Digest::SHA256.hexdigest(rows.join("\n") + "\n"), input("APK_METADATA_SHA256"), "APK metadata")
      release = probe!(image, "/bin/cat", "/etc/alpine-release").strip
      raise Failure, "native Alpine release drift" unless release == input("ALPINE_VERSION")

      hashes!(image, FILE_INPUTS)
      report = probe!(image, "/usr/bin/gdb", "--nx", "--batch", "--version")
      unless report.lines.first == "GNU gdb (GDB) #{@packages.fetch('gdb').sub(/-r\d+\z/, '')}\n"
        raise Failure, "native GDB version drift"
      end
    end

    def hashes!(image, files)
      lines = probe!(image, "/usr/bin/sha256sum", *files.values).lines(chomp: true)
      raise Failure, "native tool hash report is incomplete" unless lines.length == files.length

      files.zip(lines).each do |(key, path), line|
        raise Failure, "native tool hash drift: #{path}" unless line == "#{input(key)}  #{path}"
      end
    end

    def prepare_source!
      archive_name = File.basename(input("MUSL_SOURCE_URL"))
      archive = File.join(@directory, archive_name)
      unless File.exist?(archive) || File.symlink?(archive)
        archive = File.join(@workspace, archive_name)
        @runner.run!(["curl", "--disable", "--fail", "--silent", "--show-error", "--proto", "=https",
                      "--tlsv1.2", "--max-time", "120", "--output", archive, input("MUSL_SOURCE_URL")])
      end
      raise Failure, "musl source archive must be a regular file" unless File.lstat(archive).file?

      bytes = File.open(archive, File::RDONLY | File::NOFOLLOW) { |file| file.read }
      verify_digest!(Digest::SHA256.hexdigest(bytes), input("MUSL_SOURCE_SHA256"), "musl archive")
      root = archive_name.delete_suffix(".tar.gz")
      paths = []
      Zlib::GzipReader.wrap(StringIO.new(bytes)) do |gzip|
        Gem::Package::TarReader.new(gzip) do |tar|
          tar.each { |entry| extract_entry!(entry, root, paths) }
        end
      end
      source = File.join(@workspace, root)
      syscall = File.join(source, SYSCALL_SOURCE)
      raise Failure, "musl syscall source is missing" unless File.file?(syscall)

      verify_digest!(Digest::SHA256.file(syscall).hexdigest, input("MUSL_SYSCALL_CP_SHA256"), "musl syscall source")
      paths.reverse_each { |path| File.chmod(File.directory?(path) ? 0o555 : 0o444, path) }
      source
    end

    def extract_entry!(entry, root, paths)
      if entry.header.typeflag == "g"
        content = entry.read
        unless entry.full_name == "pax_global_header" && content.match?(/\A52 comment=[0-9a-f]{40}\n\z/)
          raise Failure, "unsupported musl archive metadata"
        end
        return
      end
      name = entry.full_name.delete_suffix("/")
      components = name.split("/", -1)
      unless components.first == root && components.all? { |part| part.match?(/\A[a-zA-Z0-9_.+-]+\z/) && !%w[. ..].include?(part) } &&
             (entry.file? || entry.directory?)
        raise Failure, "unsafe musl archive entry"
      end
      path = File.join(@workspace, name)
      raise Failure, "duplicate musl archive entry" if paths.include?(path)

      parent = File.dirname(path)
      raise Failure, "musl archive parent is missing" unless File.directory?(parent)

      entry.directory? ? Dir.mkdir(path, 0o700) : File.binwrite(path, entry.read, mode: "wx", perm: 0o600)
      paths << path
    end

    def verify_digest!(actual, expected, label)
      raise Failure, "native #{label} SHA-256 drift" unless actual == expected
    end

    def write_receipt!(image, base, source, cached:)
      receipt = File.join(@workspace, "receipt.json")
      keys = (INPUT_PATTERNS.keys + ["APK_PACKAGES"]).map { |key| "POSTGRES_NATIVE_#{key}" }
      inputs = @inputs.slice(*keys, "DOCKERFILE_FRONTEND_IMAGE", "POSTGRES_REBASELINE_IMAGE", "POSTGRES_REBASELINE_VERSION")
      data = { schema_version: 1, purpose: "test-only-native-observer", debugger_image: image,
               postgres_image: @postgres_image, postgres_image_id: base.fetch("Id"), platform: PLATFORM,
               cached:, owned_image_tag: @owned_tag, musl_source_directory: source,
               source_usage: "observer-display-only; mount read-only; never target code", inputs: inputs.sort.to_h }
      File.write(receipt, JSON.pretty_generate(data) + "\n", mode: "wx", perm: 0o400)
      receipt
    end

    def cleanup_failed_image!
      return unless @owned_tag

      result = @runner.capture(["docker", "image", "inspect", @owned_tag])
      if result.success
        image = image_record!(result.stdout)
        if image.fetch("Id") == input("DEBUGGER_IMAGE") || image.fetch("RepoDigests", []).include?(@postgres_image.delete_prefix("docker.io/library/"))
          raise Failure, "refusing to remove a protected native image; owned tag: #{@owned_tag}"
        end
        @runner.run!(["docker", "image", "rm", "--no-prune", @owned_tag])
      elsif !missing_image?(result.stderr, @owned_tag)
        raise Failure, "owned native image cleanup inspection failed: #{result.stderr.strip}"
      end
    rescue Failure => error
      raise Failure, "native image cleanup failed for #{@owned_tag}: #{error.message}"
    end

    def remove_workspace!(path)
      Find.find(path) { |entry| File.chmod(0o700, entry) if File.lstat(entry).directory? }
      FileUtils.remove_entry_secure(path)
    end
  end
end
