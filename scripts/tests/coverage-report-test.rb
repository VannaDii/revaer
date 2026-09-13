# frozen_string_literal: true

require "fileutils"
require "json"
require "open3"
require "tmpdir"

class CoverageReportTest
  RETAINED = %w[
    js/playwright/coverage.json js/release/lcov.info js-lcov.info
    scripts/ruby/coverage.json script-coverage.xml compile_commands.json
    cxxbridge/include/rust/cxx.h unrelated.bin
  ].freeze
  OWNED = %w[lcov.info llvm-cov.txt html/index.html html/stale.html].freeze
  FORMATS = %w[lcov html text].freeze
  CARGO_FIXTURE = <<~'RUBY'
    #!/usr/bin/env ruby
    require "fileutils"
    require "json"
    abort "unexpected cargo command" unless ARGV.take(2) == %w[llvm-cov report]
    abort "inclusive coverage flag missing" unless ARGV.include?("--no-default-ignore-filename-regex")
    format = %w[lcov html text].find { |name| ARGV.include?("--#{name}") }
    abort "unknown coverage format" unless format
    owned = %w[coverage/lcov.info coverage/llvm-cov.txt coverage/html]
    if !File.exist?("cargo.jsonl") && owned.any? { |path| File.exist?(path) || File.symlink?(path) }
      abort "stale Rust output reached the generator"
    end
    File.open("cargo.jsonl", "a") { |file| file.puts(JSON.generate({ format: format, arguments: ARGV })) }
    if ENV["COVERAGE_FAIL_FORMAT"] == format
      warn "injected #{format} generation failure"
      exit 42
    end
    paths = { "lcov" => "coverage/lcov.info", "html" => "coverage/html/index.html", "text" => "coverage/llvm-cov.txt" }
    path = paths.fetch(format)
    FileUtils.mkdir_p(File.dirname(path))
    File.write(path, "fresh #{format}\n")
  RUBY

  def initialize(source)
    @source = source
    @assertions = 0
    @runs = 0
  end

  def run
    Dir.mktmpdir("revaer-coverage-report-") do |temporary|
      scenario(temporary, "fresh", seed: false) { |root| success(root) }
      scenario(temporary, "populated") do |root|
        success(root)
        File.unlink(File.join(root, "cargo.jsonl"))
        write(root, "coverage/html/stale.html", "stale second run")
        success(root)
      end
      FORMATS.each do |format|
        scenario(temporary, "failed-#{format}") do |root|
          output, error, status = execute(root, format)
          check(!status.success? && error.include?("injected #{format} generation failure"), "#{format} failure propagated: #{output}#{error}")
          check(calls(root) == FORMATS.take(FORMATS.index(format) + 1), "#{format} stops subsequent generators")
          check(!File.exist?(File.join(root, "coverage/html/stale.html")), "stale HTML removed on failure")
          OWNED.each do |path|
            absolute = File.join(root, "coverage", path)
            check(!File.exist?(absolute) || !File.read(absolute).start_with?("stale"), "stale #{path} not retained after failure")
          end
        end
      end
      scenario(temporary, "linked-owned-outputs") do |root|
        %w[lcov.info llvm-cov.txt html].each do |path|
          target = File.join(root, "external", path)
          FileUtils.mkdir_p(path == "html" ? target : File.dirname(target))
          sentinel = path == "html" ? File.join(target, "keep") : target
          File.write(sentinel, "outside #{path}")
          FileUtils.rm_rf(File.join(root, "coverage", path))
          File.symlink(target, File.join(root, "coverage", path))
        end
        success(root)
        %w[lcov.info llvm-cov.txt html/keep].each do |path|
          check(File.read(File.join(root, "external", path)).start_with?("outside"), "external #{path} unchanged")
        end
      end
      scenario(temporary, "linked-directory", seed: false) do |root|
        external = File.join(root, "external")
        FileUtils.mkdir_p(File.join(external, "html"))
        File.write(File.join(external, "html", "keep"), "outside")
        File.symlink(external, File.join(root, "coverage"))
        _output, error, status = execute(root)
        check(!status.success? && error.include?("must not be a symlink"), "linked output root rejected")
        check(calls(root).empty?, "linked root never runs a generator")
        check(File.symlink?(File.join(root, "coverage")), "linked root left intact")
        check(File.read(File.join(external, "html", "keep")) == "outside", "linked root target unchanged")
      end
      scenario(temporary, "file-directory", seed: false) do |root|
        write(root, "coverage", "not a directory")
        _output, _error, status = execute(root)
        check(!status.success? && calls(root).empty?, "non-directory output root rejected before generation")
        check(File.read(File.join(root, "coverage")) == "not a directory", "non-directory output root retained")
      end
    end
    puts "Rust coverage output ownership: #{@runs} recipe runs, #{@assertions} assertions; temporary fixtures removed."
  end

  private

  def check(condition, message)
    @assertions += 1
    raise "Coverage report regression: #{message}" unless condition
  end

  def write(root, path, content)
    absolute = File.join(root, path)
    FileUtils.mkdir_p(File.dirname(absolute))
    File.write(absolute, content)
  end

  def scenario(temporary, name, seed: true)
    root = File.join(temporary, name)
    FileUtils.mkdir_p(File.join(root, "bin"))
    FileUtils.cp(File.join(@source, "justfile"), root)
    FileUtils.cp_r(File.join(@source, "just"), root)
    write(root, "bin/cargo", CARGO_FIXTURE)
    File.chmod(0o700, File.join(root, "bin/cargo"))
    if seed
      RETAINED.each { |path| write(root, "coverage/#{path}", "preserved #{path}\n") }
      OWNED.each { |path| write(root, "coverage/#{path}", "stale #{path}\n") }
    end
    yield root
    return unless seed

    RETAINED.each do |path|
      check(File.binread(File.join(root, "coverage", path)) == "preserved #{path}\n", "#{name} preserves #{path} exactly")
    end
  end

  def execute(root, failure = nil)
    @runs += 1
    Open3.capture3({ "PATH" => "#{root}/bin:#{ENV.fetch('PATH')}", "COVERAGE_FAIL_FORMAT" => failure },
      "just", "--no-dotenv", "--justfile", File.join(root, "justfile"), "--working-directory", root, "cov-report")
  end

  def calls(root)
    path = File.join(root, "cargo.jsonl")
    return [] unless File.exist?(path)

    File.readlines(path).map { |line| JSON.parse(line).fetch("format") }
  end

  def success(root)
    output, error, status = execute(root)
    check(status.success?, "report recipe succeeds: #{output}#{error}")
    check(calls(root) == FORMATS, "all report generators run in order")
    { "lcov.info" => "lcov", "html/index.html" => "html", "llvm-cov.txt" => "text" }.each do |path, format|
      check(File.read(File.join(root, "coverage", path)) == "fresh #{format}\n", "fresh #{format} replaces stale output")
    end
    check(!File.exist?(File.join(root, "coverage", "html", "stale.html")), "old HTML cannot survive regeneration")
  end
end

CoverageReportTest.new(ARGV.fetch(0)).run
