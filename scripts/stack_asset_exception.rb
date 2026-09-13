# frozen_string_literal: true

require "digest"
require "json"

module RevaerDatabaseRebaseline
  class AssetPullRequest
    REPOSITORY = { "id" => "R_kgDOQJiaFw", "nameWithOwner" => "VannaDii/revaer" }.freeze
    EXPIRED_EVENTS = %w[ClosedEvent MergedEvent ReopenedEvent].freeze
    QUERY = <<~GRAPHQL.freeze
      query($cursor: String) {
        repository(owner: "VannaDii", name: "revaer") {
          id nameWithOwner
          pullRequest(number: 130) {
            id state closed merged closedAt mergedAt updatedAt
            baseRefName baseRefOid headRefName headRefOid
            headRepository { id nameWithOwner }
            timelineItems(first: 100, after: $cursor) {
              totalCount pageInfo { hasNextPage endCursor }
              nodes { __typename ... on Node { id } }
            }
          }
        }
      }
    GRAPHQL

    class UniqueObject < Hash
      def []=(key, value)
        raise JSON::ParserError, "ASSET-1 JSON contains a duplicate field" if key?(key)

        super
      end
    end

    def self.parse_json(source)
      verify_json_decoder!
      JSON.parse(source, object_class: UniqueObject, allow_duplicate_key: false)
    end

    def self.verify_json_decoder!
      # Old JSON uses the setter; newer JSON can collapse fields before it.
      JSON.parse('{"outer":{"field":null,"\\u0066ield":true}}', object_class: UniqueObject, allow_duplicate_key: false)
      raise Failure, "ASSET-1 JSON parser lacks duplicate-field rejection"
    rescue JSON::ParserError
      nil
    end

    def initialize(root:, runner:)
      @root = root
      @runner = runner
    end

    def verify!(base_sha, head_sha)
      pages = []
      cursors = []
      node_ids = []
      signature = nil
      expected_count = nil
      cursor = nil
      loop do
        page = fetch_page(cursor)
        repository = object(object(page.fetch("data")).fetch("repository"))
        ensure!(repository.slice("id", "nameWithOwner") == REPOSITORY, "wrong repository")
        pull = object(repository.fetch("pullRequest"))
        verify_identity!(pull, base_sha, head_sha)
        current = pull.reject { |key, _| key == "timelineItems" }
        signature ||= current
        ensure!(current == signature, "PR identity changed during pagination")
        timeline = object(pull.fetch("timelineItems"))
        count = timeline.fetch("totalCount")
        ensure!(count.is_a?(Integer) && count >= 0, "invalid timeline count")
        expected_count ||= count
        ensure!(count == expected_count, "timeline changed during pagination")
        nodes = timeline.fetch("nodes")
        ensure!(nodes.is_a?(Array) && nodes.length <= 100, "invalid timeline page")
        nodes.each { |node| verify_event!(object(node), node_ids) }
        pages << page
        info = object(timeline.fetch("pageInfo"))
        more = info.fetch("hasNextPage")
        cursor = info.fetch("endCursor")
        ensure!(more == true || more == false, "invalid pagination state")
        ensure!(cursor.nil? || (cursor.is_a?(String) && !cursor.empty?), "invalid timeline cursor")
        ensure!(nodes.empty? || cursor, "missing timeline cursor")
        unless more
          ensure!(node_ids.length == expected_count, "incomplete timeline")
          return { "pull_request" => signature, "timeline_count" => expected_count, "pages" => pages }
        end
        ensure!(!nodes.empty? && node_ids.length < expected_count && cursor && !cursors.include?(cursor), "incomplete or repeated timeline page")
        cursors << cursor
      end
    rescue KeyError, JSON::ParserError, TypeError
      raise Failure, "ASSET-1 provider evidence is malformed or incomplete"
    end

    private

    def fetch_page(cursor)
      command = ["gh", "api", "--hostname", "github.com", "graphql", "-f", "query=#{QUERY}"]
      command.concat(["-f", "cursor=#{cursor}"]) if cursor
      raw = @runner.run!(command, chdir: @root)
      page = object(self.class.parse_json(raw))
      ensure!(!page.key?("errors"), "provider returned GraphQL errors")
      page
    end

    def verify_identity!(pull, base_sha, head_sha)
      expected = {
        "id" => "PR_kwDOQJiaF8749-ef", "state" => "OPEN", "closed" => false,
        "merged" => false, "closedAt" => nil, "mergedAt" => nil,
        "baseRefOid" => base_sha, "headRefOid" => head_sha,
        "headRefName" => "stack/media3-53-sonar-asset-inputs", "headRepository" => REPOSITORY
      }
      expected.each { |key, value| ensure!(pull.fetch(key) == value, "PR identity, refs or open state do not match") }
      %w[baseRefName updatedAt].each do |key|
        value = pull.fetch(key)
        ensure!(value.is_a?(String) && !value.empty?, "missing PR identity field")
      end
    end

    def verify_event!(node, node_ids)
      kind = node.fetch("__typename")
      id = node.fetch("id")
      ensure!(kind.is_a?(String) && !kind.empty? && id.is_a?(String) && !id.empty?, "malformed timeline event")
      ensure!(!EXPIRED_EVENTS.include?(kind), "exception permanently expired after close, merge or reopen")
      ensure!(!node_ids.include?(id), "duplicate timeline event")
      node_ids << id
    end

    def object(value)
      ensure!(value.is_a?(Hash), "provider evidence is not an object")
      value
    end

    def ensure!(condition, reason)
      raise Failure, "ASSET-1 #{reason}" unless condition
    end
  end

  class StackAssetException
    INVENTORY_PATH = "docs/adr/support/585-binary-deletions.json"
    INVENTORY_SHA256 = "3e1e4af169880c3aee57e967a01c58f9d4f368bb7ac9b72666622e7ce1918d7a"
    IMPLEMENTATION_PATHS = %w[
      scripts/stack_asset_exception.rb scripts/database_rebaseline/contract.rb
      scripts/database_rebaseline/support.rb scripts/database-rebaseline.rb
      docs/adr/588-first-release-decision-package.md docs/adr/support/588-decision-details.md
      .github/instructions/devops.instructions.md
    ].freeze

    def initialize(contract, runner:)
      @contract = contract
      @runner = runner
    end

    def verify!(base_sha, head_sha, binary_paths, changed_paths)
      unless binary_paths.length == 213 && binary_paths.uniq.length == 213
        raise Failure, "Git reported a binary or uncountable diff outside the complete ASSET-1 inventory"
      end
      inventory = inventory!
      unless binary_paths.sort == inventory.map { |entry| entry.fetch("path") }.sort
        raise Failure, "Git reported a binary or uncountable diff outside the complete ASSET-1 inventory"
      end
      ancestry = @runner.capture(["git", "merge-base", "--is-ancestor", base_sha, head_sha], chdir: @contract.root)
      raise Failure, "ASSET-1 requires an ancestor base" unless ancestry.success

      verify_objects!(base_sha, head_sha, inventory, changed_paths)
      verify_implementation!(head_sha)
      provider = AssetPullRequest.new(root: @contract.root, runner: @runner).verify!(base_sha, head_sha)
      {
        "decision" => "ADR 588 ASSET-1", "inventory_sha256" => INVENTORY_SHA256,
        "binary_deletions" => inventory.length,
        "original_bytes" => inventory.sum { |entry| entry.fetch("priorBytes") },
        "base" => base_sha, "head" => head_sha, "provider" => provider
      }
    rescue KeyError, JSON::ParserError, TypeError
      raise Failure, "ASSET-1 inventory or Git evidence is malformed"
    rescue SystemCallError
      raise Failure, "ASSET-1 required local evidence is unavailable"
    end

    private

    def inventory!
      source = File.read(File.join(@contract.root, INVENTORY_PATH))
      document = AssetPullRequest.parse_json(source)
      raise Failure, "ASSET-1 inventory document is malformed" unless document.is_a?(Hash)

      inventory = document.fetch("binaryDeletions")
      unless inventory.is_a?(Array) && inventory.length == 213 &&
             Digest::SHA256.hexdigest(JSON.generate(inventory)) == INVENTORY_SHA256
        raise Failure, "ASSET-1 binary inventory differs from the approved identity"
      end
      inventory
    end

    def verify_objects!(base_sha, head_sha, inventory, changed_paths)
      raw = @runner.run!(
        ["git", "diff", "--no-ext-diff", "--no-textconv", "--no-renames", "--raw", "--abbrev=40", "-z", base_sha, head_sha, "--"],
        chdir: @contract.root
      )
      fields = raw.split("\0", -1)
      unless fields.pop == "" && fields.length.even?
        raise Failure, "ASSET-1 raw Git diff is incomplete"
      end
      entries = {}
      fields.each_slice(2) do |header, path|
        if path.empty? || entries.key?(path)
          raise Failure, "ASSET-1 raw Git paths are missing or duplicated"
        end
        unless header.match?(/\A:[0-7]{6} [0-7]{6} [0-9a-f]{40} [0-9a-f]{40} [ADMT]\z/)
          raise Failure, "ASSET-1 raw Git metadata is malformed"
        end
        entries[path] = header
      end
      unless entries.keys.sort == changed_paths.sort
        raise Failure, "ASSET-1 raw and changed-line Git paths disagree"
      end
      blobs = {}
      inventory.each do |entry|
        expected = ":#{entry.fetch('priorMode')} 000000 #{entry.fetch('priorBlobOid')} #{'0' * 40} D"
        unless entries.fetch(entry.fetch("path")) == expected
          raise Failure, "ASSET-1 deletion path, mode or object does not match"
        end
        oid = entry.fetch("priorBlobOid")
        blob = blobs[oid] ||= @runner.run!(["git", "cat-file", "blob", oid], chdir: @contract.root)
        unless blob.bytesize == entry.fetch("priorBytes") && Digest::SHA256.hexdigest(blob) == entry.fetch("priorSha256")
          raise Failure, "ASSET-1 binary content does not match"
        end
      end
    end

    def verify_implementation!(head_sha)
      (IMPLEMENTATION_PATHS + [INVENTORY_PATH]).each do |path|
        checked = @runner.capture(["git", "show", "#{head_sha}:#{path}"], chdir: @contract.root)
        unless checked.success && checked.stdout == File.binread(File.join(@contract.root, path))
          raise Failure, "ASSET-1 checked head must contain this guard, consent and scoped instructions; unpublished candidates cannot receive a canonical pass"
        end
      end
    end
  end
end
