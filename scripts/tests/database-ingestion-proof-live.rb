# frozen_string_literal: true

require_relative "../database_rebaseline/final_proof"
require_relative "../database_rebaseline/ingestion_proof"

module RevaerDatabaseRebaseline
  # Narrow local driver; the parent owns canonical FinalProof integration.
  class IngestionProofLive < FinalProof
    include IngestionProof

    def run_ingestion!(candidate_path, corrections_only: false)
      @contract.freeze!
      @candidate = File.binread(candidate_path)
      @contract.verify_candidate_source!(@candidate)
      @contract.validate_output_path!
      FileUtils.mkdir_p(@contract.output_path, mode: 0o700)
      File.binwrite(@contract.candidate_path, @candidate)
      final = FinalSql.new(@contract).verify!
      started = false
      begin
        @runner.run!([
          "docker", "run", "-d", "--name", @container, "--shm-size", "1g",
          "--publish", "127.0.0.1::5432",
          "-e", "POSTGRES_HOST_AUTH_METHOD=trust",
          "-e", "POSTGRES_INITDB_ARGS=--locale=C --encoding=UTF8 --data-checksums",
          "-e", "TZ=UTC", @contract.postgres_image
        ])
        started = true
        wait_ready!
        provision!
        apply!("reference_proof", @candidate, role: "postgres")
        apply!(@database, final)
        seal!
        verify_ingestion_corrections!
        verify_ingestion_parity! unless corrections_only
        raise Failure, "ingestion correction regression failed: #{@failures.join('; ')}" unless @failures.empty?
      ensure
        @runner.run!(["docker", "rm", "-fv", @container]) if started
      end
    end
  end
end

if $PROGRAM_NAME == __FILE__
  begin
    corrections_only = ARGV.delete("--corrections-only") == "--corrections-only"
    raise RevaerDatabaseRebaseline::Failure, "provide the pinned frozen candidate path" unless ARGV.length == 1

    RevaerDatabaseRebaseline::IngestionProofLive.new.run_ingestion!(ARGV.fetch(0), corrections_only:)
  rescue RevaerDatabaseRebaseline::Failure => error
    warn "database-ingestion-proof: #{error.message}"
    exit 1
  end
end
