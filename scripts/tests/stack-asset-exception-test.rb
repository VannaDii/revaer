# frozen_string_literal: true

require_relative "../database_rebaseline/support"
require_relative "../database_rebaseline/contract"
require "fileutils"
require "tmpdir"

module StackAssetExceptionTest
  ROOT = File.expand_path("../..", __dir__)
  BASE = "1" * 40
  HEAD = "2" * 40
  INVENTORY = JSON.parse(File.read(File.join(ROOT, RevaerDatabaseRebaseline::StackAssetException::INVENTORY_PATH))).fetch("binaryDeletions")

  class Assertions
    attr_reader :count

    def initialize
      @count = 0
    end

    def equal(expected, actual, label)
      @count += 1
      raise "#{label}: expected #{expected.inspect}, got #{actual.inspect}" unless expected == actual
    end

    def failure(pattern, label)
      @count += 1
      yield
    rescue RevaerDatabaseRebaseline::Failure => error
      raise "#{label}: wrong failure #{error.message}" unless error.message.match?(pattern)
    else
      raise "#{label}: expected failure"
    end
  end

  class Contract
    attr_reader :root

    def initialize(root = ROOT)
      @root = root
    end

    def changed_line_limit(scope)
      raise RevaerDatabaseRebaseline::Failure, "unknown scope" unless %w[stack init-assembly].include?(scope)

      9999
    end
  end

  class Runner
    attr_accessor :numstat, :raw, :ancestor, :missing_checked_file, :changed_blob
    attr_reader :commands, :pages

    def initialize(pages:, blobs:)
      @pages = pages.dup
      @blobs = blobs
      @ancestor = true
      @commands = []
      @numstat = "3\t4\tdocs/ordinary-text.md\0" + INVENTORY.map { |entry| "-\t-\t#{entry.fetch('path')}\0" }.join
      @raw = ":000000 100644 #{'0' * 40} #{'a' * 40} A\0docs/ordinary-text.md\0" + INVENTORY.map do |entry|
        ":#{entry.fetch('priorMode')} 000000 #{entry.fetch('priorBlobOid')} #{'0' * 40} D\0#{entry.fetch('path')}\0"
      end.join
    end

    def capture(command, **options)
      if command[0, 3] == ["git", "merge-base", "--is-ancestor"]
        @commands << command
        return result("", @ancestor)
      end
      if command[0, 2] == ["git", "rev-parse"]
        @commands << command
        return result(command.last.delete_suffix("^{commit}") + "\n", true)
      end
      if command[0, 2] == ["git", "show"]
        @commands << command
        path = command.fetch(2).delete_prefix("#{HEAD}:")
        return result("", false) if path == @missing_checked_file

        return result(File.binread(File.join(ROOT, path)), true)
      end
      result(run!(command, **options), true)
    end

    def run!(command, **_options)
      @commands << command
      return @numstat if command.include?("--numstat")
      return @raw if command.include?("--raw")
      if command[0, 3] == ["git", "cat-file", "blob"]
        return @blobs.fetch(command.fetch(3)) + (@changed_blob ? "changed" : "")
      end
      unless command[0, 5] == ["gh", "api", "--hostname", "github.com", "graphql"]
        raise "unexpected command: #{command.inspect}"
      end
      raise RevaerDatabaseRebaseline::Failure, "provider unavailable" if @pages.empty?

      page = @pages.shift
      page.is_a?(String) ? page : JSON.generate(page)
    end

    private

    def result(stdout, success)
      RevaerDatabaseRebaseline::CommandRunner::Result.new(stdout:, stderr: "", success:)
    end
  end

  module_function

  def page(nodes: [], total: nodes.length, more: false, cursor: nodes.empty? ? nil : "last", updated: "2026-09-12T00:00:00Z", base: BASE, head: HEAD)
    {
      "data" => { "repository" => {
        "id" => "R_kgDOQJiaFw", "nameWithOwner" => "VannaDii/revaer",
        "pullRequest" => {
          "id" => "PR_kwDOQJiaF8749-ef", "state" => "OPEN", "closed" => false,
          "merged" => false, "closedAt" => nil, "mergedAt" => nil, "updatedAt" => updated,
          "baseRefName" => "stack/prerequisite", "baseRefOid" => base,
          "headRefName" => "stack/media3-53-sonar-asset-inputs", "headRefOid" => head,
          "headRepository" => { "id" => "R_kgDOQJiaFw", "nameWithOwner" => "VannaDii/revaer" },
          "timelineItems" => { "totalCount" => total, "nodes" => nodes,
            "pageInfo" => { "hasNextPage" => more, "endCursor" => cursor } }
        }
      } }
    }
  end

  def node(index, kind = "IssueComment")
    { "id" => "timeline-#{index}", "__typename" => kind }
  end

  def pull(payload)
    payload.fetch("data").fetch("repository").fetch("pullRequest")
  end

  def provider(runner)
    RevaerDatabaseRebaseline::AssetPullRequest.new(root: ROOT, runner:)
  end

  def guard(runner)
    RevaerDatabaseRebaseline::ChangedLineGuard.new(Contract.new, runner:)
  end

  def test_provider(assertions)
    runner = Runner.new(pages: [page], blobs: {})
    receipt = provider(runner).verify!(BASE, HEAD)
    assertions.equal(0, receipt.fetch("timeline_count"), "empty complete timeline")
    assertions.equal(1, runner.commands.length, "one fresh provider call")
    assertions.equal(false, runner.commands.first.join.include?("itemTypes"), "unfiltered timeline required")
    assertions.failure(/unavailable/, "previous receipt is not cached") { provider(runner).verify!(BASE, HEAD) }

    first = page(nodes: (0...100).map { |index| node(index) }, total: 101, more: true, cursor: "page-1")
    last = page(nodes: [node(100)], total: 101, cursor: "page-2")
    runner = Runner.new(pages: [first, last], blobs: {})
    assertions.equal(101, provider(runner).verify!(BASE, HEAD).fetch("timeline_count"), "complete pagination")
    assertions.equal("cursor=page-1", runner.commands.last.last, "opaque cursor forwarded")
    mutations = [
      ["wrong repository", ->(value) { value.fetch("data").fetch("repository")["id"] = "other" }],
      ["wrong PR", ->(value) { pull(value)["id"] = "other" }],
      ["wrong base", ->(value) { pull(value)["baseRefOid"] = "3" * 40 }],
      ["wrong head", ->(value) { pull(value)["headRefOid"] = "3" * 40 }],
      ["wrong branch", ->(value) { pull(value)["headRefName"] = "stack/other" }],
      ["fork", ->(value) { pull(value)["headRepository"]["id"] = "fork" }],
      ["closed", ->(value) { pull(value)["state"] = "CLOSED" }],
      ["closed flag", ->(value) { pull(value)["closed"] = true }],
      ["merged flag", ->(value) { pull(value)["merged"] = true }],
      ["close timestamp", ->(value) { pull(value)["closedAt"] = "2026-09-12T00:00:00Z" }],
      ["merge timestamp", ->(value) { pull(value)["mergedAt"] = "2026-09-12T00:00:00Z" }],
      ["missing field", ->(value) { pull(value).delete("closedAt") }],
      ["partial error", ->(value) { value["errors"] = [{ "message" => "partial" }] }],
      ["filtered count", ->(value) { pull(value)["timelineItems"]["totalCount"] = 26 }],
      ["missing page", ->(value) { pull(value)["timelineItems"]["pageInfo"]["hasNextPage"] = true }],
      ["invalid more", ->(value) { pull(value)["timelineItems"]["pageInfo"]["hasNextPage"] = "false" }],
      ["invalid count", ->(value) { pull(value)["timelineItems"]["totalCount"] = -1 }]
    ]
    mutations.each do |label, mutation|
      value = page
      mutation.call(value)
      assertions.failure(/ASSET-1/, label) { provider(Runner.new(pages: [value], blobs: {})).verify!(BASE, HEAD) }
    end
    %w[ClosedEvent MergedEvent ReopenedEvent].each do |kind|
      value = page(nodes: [node(100, kind)], total: 101)
      assertions.failure(/permanently expired/, "#{kind} on final page") do
        provider(Runner.new(pages: [first, value], blobs: {})).verify!(BASE, HEAD)
      end
    end
    ["not json", "[]", '{"data":null,"data":null}'].each do |raw|
      assertions.failure(/ASSET-1/, "malformed provider document") do
        provider(Runner.new(pages: [raw], blobs: {})).verify!(BASE, HEAD)
      end
    end
    final_mutations = [
      page(nodes: [node(100)], total: 101, updated: "2026-09-12T00:00:01Z"),
      page(nodes: [node(100)], total: 102), page(nodes: [node(99)], total: 101),
      page(nodes: [node(100)], total: 103, more: true, cursor: "page-1"),
      page(nodes: [node(100)], total: 101, cursor: nil),
      page(nodes: [{ "__typename" => "IssueComment" }], total: 101),
      page(nodes: [nil], total: 101)
    ]
    final_mutations.each do |last_page|
      assertions.failure(/ASSET-1/, "malformed, repeated or changed final page") do
        provider(Runner.new(pages: [first, last_page], blobs: {})).verify!(BASE, HEAD)
      end
    end
    assertions.failure(/unavailable/, "failed later page is not complete history") do
      provider(Runner.new(pages: [first], blobs: {})).verify!(BASE, HEAD)
    end
  end

  def test_guard(assertions)
    actual_git = RevaerDatabaseRebaseline::CommandRunner.new
    blobs = INVENTORY.map { |entry| entry.fetch("priorBlobOid") }.uniq.to_h do |oid|
      [oid, actual_git.run!(["git", "cat-file", "blob", oid], chdir: ROOT)]
    end
    runner = Runner.new(pages: [page], blobs:)
    instance = guard(runner)
    assertions.equal([3, 4, 7, 9999], instance.verify!("stack", BASE, HEAD), "all text counted in exact model")
    receipt = instance.asset_exception_evidence
    assertions.equal(213, receipt.fetch("binary_deletions"), "all binary entries verified")
    assertions.equal(21_064_113, receipt.fetch("original_bytes"), "exact original bytes")
    assertions.equal(RevaerDatabaseRebaseline::StackAssetException::INVENTORY_SHA256, receipt.fetch("inventory_sha256"), "exact inventory digest")
    assertions.equal(true, runner.commands.find { |command| command.include?("--numstat") }.include?("-z"), "NUL-safe paths")

    [9999, 10000].each do |total|
      runner = Runner.new(pages: [page], blobs:)
      runner.numstat.sub!("3\t4\t", "#{total}\t0\t")
      if total == 9999
        assertions.equal([9999, 0, 9999, 9999], guard(runner).verify!("stack", BASE, HEAD), "exact text ceiling")
      else
        assertions.failure(/10000 changed lines/, "text over ceiling") { guard(runner).verify!("stack", BASE, HEAD) }
        assertions.equal(0, runner.commands.count { |command| command.first == "gh" }, "over-limit text never queries provider")
      end
    end
    mutations = [
      ["missing binary", ->(value) { value.numstat.sub!(/-\t-\t[^\0]+\0/, "") }],
      ["extra binary", ->(value) { value.numstat << "-\t-\textra.bin\0" }],
      ["wrong binary", ->(value) { value.numstat.sub!(INVENTORY.first.fetch("path"), "wrong.bin") }],
      ["binary rename/add", ->(value) { value.raw.sub!(/ D\0/, " A\0") }],
      ["wrong mode", ->(value) { value.raw.sub!(":100644", ":120000") }],
      ["wrong blob", ->(value) { value.raw.sub!(INVENTORY.first.fetch("priorBlobOid"), "f" * 40) }],
      ["missing raw", ->(value) { value.raw.sub!(/[^\0]+\0[^\0]+\0/, "") }],
      ["incomplete raw", ->(value) { value.raw.chop! }],
      ["invalid raw metadata", ->(value) { value.raw.sub!(":000000", ":broken") }],
      ["missing text numstat", ->(value) { value.numstat.sub!("3\t4\tdocs/ordinary-text.md\0", "") }],
      ["duplicate raw", ->(value) { value.raw << value.raw.split("\0", -1).first(2).join("\0") + "\0" }],
      ["binary byte drift", ->(value) { value.changed_blob = true }],
      ["non-ancestor", ->(value) { value.ancestor = false }]
    ]
    mutations.each do |label, mutation|
      runner = Runner.new(pages: [page], blobs:)
      mutation.call(runner)
      assertions.failure(/ASSET-1/, label) { guard(runner).verify!("stack", BASE, HEAD) }
    end
    RevaerDatabaseRebaseline::StackAssetException::IMPLEMENTATION_PATHS.each do |path|
      runner = Runner.new(pages: [page], blobs:)
      runner.missing_checked_file = path
      assertions.failure(/checked head/, "unpublished #{path}") { guard(runner).verify!("stack", BASE, HEAD) }
    end
    runner = Runner.new(pages: [], blobs: {})
    runner.numstat = "1\t2\twith\ttab\nand-newline\0"
    instance = guard(runner)
    assertions.equal([1, 2, 3, 9999], instance.verify!("stack", BASE, HEAD), "text path bytes preserved")
    assertions.equal(nil, instance.asset_exception_evidence, "ordinary text has no exception")
    ["1\t2\tpath", "1\t2\t\0", "-1\t2\tpath\0", "-\t2\tpath\0", "1\t2\tx\0" * 2].each do |output|
      runner.numstat = output
      assertions.failure(/Git/, "malformed numstat") { instance.verify!("stack", BASE, HEAD) }
    end
    runner.numstat = "-\t-\tone.bin\0"
    assertions.failure(/binary or uncountable/, "init assembly has no exception") { instance.verify!("init-assembly", BASE, HEAD) }
    runner.numstat = ""
    assertions.equal([0, 0, 0, 9999], instance.verify!("stack", BASE, HEAD), "empty diff")
  end

  def test_inventory_integrity(assertions)
    Dir.mktmpdir("revaer-asset-inventory-test.") do |root|
      path = File.join(root, RevaerDatabaseRebaseline::StackAssetException::INVENTORY_PATH)
      FileUtils.mkdir_p(File.dirname(path))
      contract = Contract.new(root)
      runner = Runner.new(pages: [], blobs: {})
      validator = RevaerDatabaseRebaseline::StackAssetException.new(contract, runner:)
      paths = INVENTORY.map { |entry| entry.fetch("path") }
      assertions.failure(/unavailable/, "missing pinned inventory") { validator.verify!(BASE, HEAD, paths, paths) }
      ["null", "[]", "{}", "not-json", '{"binaryDeletions":[],"binaryDeletions":[]}'].each do |source|
        File.write(path, source)
        assertions.failure(/ASSET-1/, "invalid inventory document") { validator.verify!(BASE, HEAD, paths, paths) }
      end
      INVENTORY.first.each_key do |key|
        changed = JSON.parse(JSON.generate(INVENTORY))
        changed.first[key] = key == "priorBytes" ? 0 : "changed"
        File.write(path, JSON.generate({ "binaryDeletions" => changed }))
        assertions.failure(/approved identity/, "pinned #{key} drift") { validator.verify!(BASE, HEAD, paths, paths) }
      end
      File.write(path, JSON.generate({ "binaryDeletions" => INVENTORY.reverse }))
      assertions.failure(/approved identity/, "canonical inventory order drift") { validator.verify!(BASE, HEAD, paths, paths) }
      assertions.equal([], runner.commands, "invalid inventory never queries GitHub or Git")
    end
  end

  class GitRunner < RevaerDatabaseRebaseline::CommandRunner
    attr_accessor :provider_page

    def run!(command, **options)
      return JSON.generate(@provider_page) if command.first == "gh"

      super
    end
  end

  def test_real_git_boundary(assertions)
    actual = RevaerDatabaseRebaseline::CommandRunner.new
    blobs = INVENTORY.map { |entry| entry.fetch("priorBlobOid") }.uniq.to_h do |oid|
      [oid, actual.run!(["git", "cat-file", "blob", oid], chdir: ROOT)]
    end
    Dir.mktmpdir("revaer-asset-git-test.") do |root|
      git = ->(*args) { actual.run!(["git", *args], chdir: root).strip }
      git.call("init", "-q", "--initial-branch=main")
      git.call("config", "user.name", "Revaer Test")
      git.call("config", "user.email", "revaer-test@example.invalid")
      files = RevaerDatabaseRebaseline::StackAssetException::IMPLEMENTATION_PATHS +
              [RevaerDatabaseRebaseline::StackAssetException::INVENTORY_PATH]
      files.each do |path|
        destination = File.join(root, path)
        FileUtils.mkdir_p(File.dirname(destination))
        FileUtils.cp(File.join(ROOT, path), destination)
      end
      INVENTORY.each do |entry|
        destination = File.join(root, entry.fetch("path"))
        FileUtils.mkdir_p(File.dirname(destination))
        File.binwrite(destination, blobs.fetch(entry.fetch("priorBlobOid")))
      end
      text_path = File.join(root, "counted.txt")
      File.write(text_path, "one\n")
      git.call("add", ".")
      git.call("commit", "-q", "-m", "test(media): stage approved asset inputs")
      base = git.call("rev-parse", "HEAD")
      INVENTORY.each { |entry| File.unlink(File.join(root, entry.fetch("path"))) }
      File.write(text_path, "one\ntwo\n")
      git.call("add", "--all")
      git.call("commit", "-q", "-m", "test(media): delete exact approved inventory")
      head = git.call("rev-parse", "HEAD")
      runner = GitRunner.new
      runner.provider_page = page(base:, head:)
      checked = RevaerDatabaseRebaseline::ChangedLineGuard.new(Contract.new(root), runner:)
      assertions.equal([1, 0, 1, 9999], checked.verify!("stack", base, head), "real Git blobs, raw records and complete checked-head files")
      File.write(text_path, "one\ntwo\nthree\n")
      git.call("add", "counted.txt")
      git.call("commit", "-q", "-m", "test(media): count subsequent text corrections")
      next_head = git.call("rev-parse", "HEAD")
      assertions.failure(/refs/, "stale provider head cannot qualify new text") { checked.verify!("stack", base, next_head) }
      assertions.equal(nil, checked.asset_exception_evidence, "failed retry clears earlier receipt")
      runner.provider_page = page(base:, head: next_head)
      assertions.equal([2, 0, 2, 9999], checked.verify!("stack", base, next_head), "new exact provider pair retains full text accounting")
      assertions.equal(213, checked.asset_exception_evidence.fetch("binary_deletions"), "complete deletion set persists across text correction")
    end
  end

  def run
    assertions = Assertions.new
    test_provider(assertions)
    test_guard(assertions)
    test_inventory_integrity(assertions)
    test_real_git_boundary(assertions)
    puts "stack-asset-exception-test: #{assertions.count} assertions passed; provider/head controls are synthetic, not canonical PR acceptance"
  end
end

StackAssetExceptionTest.run
