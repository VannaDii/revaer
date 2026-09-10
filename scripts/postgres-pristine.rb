# frozen_string_literal: true

require_relative "postgres_pristine/container"
require_relative "postgres_pristine/snapshot"

if $PROGRAM_NAME == __FILE__
  begin
    root = File.expand_path("..", __dir__)
    mode = ARGV.fetch(0, "")
    raise RevaerPostgresPristine::Failure, "expected generate or validate" unless %w[generate validate].include?(mode) && ARGV.length == 1

    RevaerPostgresPristine::Container.new(root).with_running do |container|
      database, owner = container.provision(SecureRandom.hex(4))
      snapshot = RevaerPostgresPristine::Snapshot.new(container, database, owner)
      bytes = snapshot.read
      path = snapshot.write_evidence(root, bytes)
      expected_path = File.join(root, "config/postgres-pristine-16.14.tsv")
      if mode == "validate"
        raise RevaerPostgresPristine::Failure, "committed pristine snapshot is missing" unless File.file?(expected_path)
        raise RevaerPostgresPristine::Failure, "committed pristine snapshot differs from pinned image" unless File.binread(expected_path) == bytes
      end
      puts "pristine #{mode}: #{bytes.lines.count} lines; #{bytes.bytesize} bytes; SHA-256 #{Digest::SHA256.hexdigest(bytes)}"
      puts "complete generated evidence: #{path}"
      puts "snapshot exceeds the 9,999-line PR ceiling; do not commit an incomplete snapshot" if bytes.lines.count > 9_999
    end
  rescue RevaerPostgresPristine::Failure => error
    warn error.message
    exit 1
  end
end
