# frozen_string_literal: true

require "digest"
require "fileutils"
require_relative "query"

module RevaerPostgresPristine
  class Snapshot
    IDENTITY = {
      "kind" => "identity", "database" => "<database>", "owner" => "<database_owner>",
      "server_version_num" => "160014", "encoding" => "UTF8", "collate" => "C", "ctype" => "C",
      "integer_datetimes" => "on", "standard_conforming_strings" => "on", "data_checksums" => "on",
      "owner_valid" => true
    }.freeze

    attr_reader :inventory

    def initialize(container, database, owner)
      @container, @database, @owner = container, database, owner
      @query = Query.new
      @inventory = parse(sql(@query.inventory_sql)).to_h { |row| [row.fetch("catalog"), row.fetch("columns")] }
      raise Failure, "missing pristine catalog inventory" unless inventory.keys.sort == COLUMNS.keys.sort
    end

    def read(prefix: "")
      records = parse(sql(batch(prefix)))
      raise Failure, "unsupported pristine PostgreSQL or owner identity" unless records.shift == IDENTITY
      raise Failure, "incomplete pristine catalog result" unless records.map { |row| row.fetch("catalog") } == COLUMNS.keys

      output = ["#postgres-pristine\t16.14\t<database>\t<database_owner>\n"]
      records.each do |record|
        catalog = record.fetch("catalog")
        rows = record.fetch("rows")
        output << "#catalog\t#{catalog}\t#{rows.length}\n"
        rows.each do |row|
          validate_values!(row)
          fields = row.sort.map { |key, value| "#{key}=#{JSON.generate(value)}" }
          output << "#{catalog}\t#{fields.join("\t")}\n"
        end
      end
      raise Failure, "duplicate catalog row evidence" unless output.uniq.length == output.length

      output.sort_by(&:b).join
    end

    def write_evidence(root, bytes)
      directory = File.join(root, "target/postgres-pristine")
      FileUtils.mkdir_p(directory, mode: 0o700)
      File.chmod(0o700, directory)
      path = File.join(directory, "postgres-pristine-16.14.tsv")
      write_private(path, bytes)
      evidence = {
        "postgres_image" => @container.image, "postgres_version" => @container.version,
        "image_identity" => @container.identity, "database_identity" => IDENTITY,
        "snapshot_sha256" => Digest::SHA256.hexdigest(bytes), "snapshot_bytes" => bytes.bytesize,
        "snapshot_lines" => bytes.lines.count, "catalog_count" => COLUMNS.length,
        "columns" => inventory, "excluded_columns" => EXCLUDED,
        "oid_handling" => "row OIDs omitted; references resolved; TOAST identities derive from parent relation",
        "reader" => "direct constrained database owner connection; no role substitution or extra grants",
        "transaction" => "REPEATABLE READ READ ONLY; fixed pg_catalog search_path",
        "generated_at_utc" => Time.now.utc.strftime("%Y-%m-%dT%H:%M:%SZ")
      }
      write_private(File.join(directory, "provenance.json"), "#{JSON.pretty_generate(evidence)}\n")
      path
    end

    private

    def write_private(path, bytes)
      raise Failure, "evidence destination must not be a symlink" if File.symlink?(path)

      File.open(path, File::WRONLY | File::CREAT | File::TRUNC, 0o600) { |file| file.write(bytes) }
      File.chmod(0o600, path)
    end

    def validate_values!(row)
      encoded = JSON.generate(row)
      raise Failure, "unresolved catalog identity" if encoded.include?("<unresolved_reference>")
      if row.key?("probin") && row["probin"] && !row["probin"].match?(%r{\A\$libdir/[a-zA-Z0-9_-]+\z})
        raise Failure, "catalog requires an unrepresentable filesystem library location"
      end
      raise Failure, "catalog output retained a raw temporary identity" if encoded.match?(/pg_(?:toast_)?temp_[0-9]+/)
      raise Failure, "catalog output retained an OID-derived TOAST name" if encoded.match?(/pg_toast_[0-9]+/)
    end

    def batch(prefix)
      statements = COLUMNS.keys.map do |catalog|
        query = @query.catalog_sql(catalog, inventory.fetch(catalog))
        if %w[pg_subscription pg_user_mapping].include?(catalog)
          # Credential-bearing columns are restricted. Prove emptiness using readable OIDs;
          # populated catalogs must attempt the full projection, never substitute masked options.
          empty = "SELECT json_build_object('kind','rows','catalog','#{catalog}','rows','[]'::json);"
          existence = catalog == "pg_user_mapping" ? "SELECT umid FROM pg_user_mappings" : "SELECT oid FROM pg_subscription"
          "SELECT CASE WHEN NOT EXISTS (#{existence}) THEN #{literal(empty)} " \
            "ELSE #{literal(query)} END\n\\gexec\n"
        else
          query
        end
      end
      <<~SQL
        BEGIN ISOLATION LEVEL REPEATABLE READ;
        SET LOCAL search_path = pg_catalog;
        SET LOCAL statement_timeout = '120s';
        SET LOCAL lock_timeout = '120s';
        SET LOCAL idle_in_transaction_session_timeout = '30s';
        #{prefix}
        SET TRANSACTION READ ONLY;
        #{@query.identity_sql}
        #{statements.join("\n")}
        ROLLBACK;
      SQL
    end

    def literal(value)
      "'#{value.gsub("'", "''")}'"
    end

    def sql(value)
      @container.sql!(value, database: @database, user: @owner)
    end

    def parse(value)
      value.lines.map { |line| JSON.parse(line) }
    rescue JSON::ParserError
      raise Failure, "invalid pristine catalog transport"
    end
  end
end
