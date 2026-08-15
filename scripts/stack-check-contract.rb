#!/usr/bin/env ruby
# frozen_string_literal: true

require "yaml"

workflow_path = ENV.fetch("REVAER_PR_WORKFLOW_PATH", ".github/workflows/pr.yml")
workflow = YAML.safe_load(File.read(workflow_path), aliases: true)
jobs = workflow.fetch("jobs")
failures = []

normalize_needs = lambda do |job|
  needs = job["needs"]
  needs.is_a?(Array) ? needs : [needs].compact
end

find_step = lambda do |job, name|
  job.fetch("steps", []).find { |step| step["name"] == name }
end

media = jobs["media-conversion"]
if media.nil?
  failures << "media-conversion job is missing"
else
  failures << "media-conversion must publish the Media Conversion Fixtures context" unless media["name"] == "Media Conversion Fixtures"

  cleanup = find_step.call(media, "Clean media fixtures")
  unless cleanup && cleanup["if"] == "always()" && cleanup["run"] == "just clean-test-fixtures"
    failures << "media-conversion cleanup must always run just clean-test-fixtures"
  end

  upload = find_step.call(media, "Upload media conversion report")
  unless upload && upload["if"] == "always()" &&
         upload.fetch("uses", "").start_with?("actions/upload-artifact@") &&
         upload.dig("with", "path") == "target/media-conversion-report.md" &&
         upload.dig("with", "if-no-files-found") == "error"
    failures << "media-conversion must always upload a required nonempty report"
  end

  conversion = find_step.call(media, "Media conversion fixtures")
  failures << "media-conversion must run the canonical conversion recipe" unless conversion && conversion["run"] == "just test-media-conversion"
end

supply = jobs["supply-chain"]
if supply.nil?
  failures << "supply-chain job is missing"
else
  failures << "supply-chain must publish the Supply Chain Checks context" unless supply["name"] == "Supply Chain Checks"
  failures << "supply-chain must run under if: always()" unless supply["if"] == "always()"
  failures << "supply-chain needs must be audit, deny, and udeps" unless normalize_needs.call(supply).sort == %w[audit deny udeps]

  expected_environment = {
    "AUDIT_RESULT" => "${{ needs.audit.result }}",
    "DENY_RESULT" => "${{ needs.deny.result }}",
    "UDEPS_RESULT" => "${{ needs.udeps.result }}"
  }
  verify = find_step.call(supply, "Verify supply chain results")
  unless verify && verify["run"] == "just verify-supply-chain-results" && verify["env"] == expected_environment
    failures << "supply-chain must pass every upstream result to the canonical verifier"
  end
end

images = jobs["build-pr-images"]
unless images && normalize_needs.call(images).include?("media-conversion")
  failures << "build-pr-images must depend on media-conversion"
end

release = jobs["build-release"]
if release.nil?
  failures << "build-release job is missing"
else
  failures << "build-release must run on pull requests without a job-level condition" if release.key?("if")
  failures << "build-release must depend on media-conversion" unless normalize_needs.call(release).include?("media-conversion")
end

unless failures.empty?
  failures.each { |failure| warn "stack-check-contract: #{failure}" }
  exit 1
end

puts "stack-check-contract: required pull-request contexts are structurally fail-closed"
