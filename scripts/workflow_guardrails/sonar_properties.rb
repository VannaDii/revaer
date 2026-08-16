# frozen_string_literal: true

require "set"
require "workflow_guardrails/input_loader"

module WorkflowGuardrails
  class SonarProperties
    BASE_KEYS = Set.new(%w[
      sonar.projectKey sonar.organization sonar.sourceEncoding
      sonar.scanner.excludeHiddenFiles sonar.text.activate
      sonar.text.inclusions.activate sonar.text.inclusions sonar.html.file.suffixes
      sonar.tsql.file.suffixes sonar.plsql.file.suffixes sonar.plsql.defaultSchema
      sonar.featureflag.cloud-security-enable-generic-yaml-and-json-analyzer
      sonar.yaml.activate sonar.json.activate sonar.lang.patterns.kubernetes
      sonar.lang.patterns.yaml sonar.sources sonar.scm.exclusions.disabled
      sonar.scm.disabled sonar.scm.provider sonar.scm.forceReloadAll
      sonar.sensor.cache.project.enable sonar.scanner.keepReport
      sonar.exclusions sonar.inclusions sonar.tests sonar.test.exclusions
      sonar.test.inclusions sonar.coverage.exclusions sonar.cpd.exclusions
      sonar.issue.ignore.multicriteria sonar.issue.ignore.allfile
      sonar.issue.ignore.block sonar.issue.enforce.multicriteria sonar.filesize.limit
      sonar.javascript.exclusions sonar.javascript.maxFileSize sonar.javascript.detectBundles
      sonar.sca.enabled sonar.sca.exclusions sonar.sca.allowManifestFailures
      sonar.sca.goNoResolve sonar.sca.mavenNoResolve sonar.sca.gradleNoResolve
      sonar.sca.pythonNoResolve sonar.sca.npmNoResolve sonar.sca.nugetNoResolve
      sonar.sca.cfamily sonar.rust.clippy.enabled sonar.rust.lcov.reportPaths
      sonar.javascript.lcov.reportPaths sonar.coverageReportPaths
      sonar.cfamily.compile-commands sonar.cfamily.llvm-cov.reportPath
      sonar.newCode.referenceBranch sonar.qualitygate.wait sonar.qualitygate.timeout
    ]).freeze
    EMPTY_KEYS = Set.new(%w[
      sonar.exclusions sonar.inclusions sonar.test.exclusions sonar.test.inclusions
      sonar.coverage.exclusions sonar.cpd.exclusions sonar.issue.ignore.multicriteria
      sonar.issue.ignore.allfile sonar.issue.ignore.block sonar.issue.enforce.multicriteria
      sonar.javascript.exclusions sonar.sca.exclusions
    ]).freeze
    REQUIRED_VALUES = {
      "sonar.projectKey" => "VannaDii_Revaer",
      "sonar.organization" => "vannadii",
      "sonar.sourceEncoding" => "UTF-8",
      "sonar.scanner.excludeHiddenFiles" => "false",
      "sonar.text.activate" => "true",
      "sonar.text.inclusions.activate" => "true",
      "sonar.text.inclusions" => "**/*",
      "sonar.html.file.suffixes" => ".html",
      "sonar.tsql.file.suffixes" => ".tsql",
      "sonar.plsql.file.suffixes" => ".plsql",
      "sonar.plsql.defaultSchema" => "public",
      "sonar.featureflag.cloud-security-enable-generic-yaml-and-json-analyzer" => "true",
      "sonar.yaml.activate" => "true",
      "sonar.json.activate" => "true",
      "sonar.scm.exclusions.disabled" => "true",
      "sonar.scm.disabled" => "false",
      "sonar.scm.provider" => "git",
      "sonar.scm.forceReloadAll" => "true",
      "sonar.sensor.cache.project.enable" => "true",
      "sonar.scanner.keepReport" => "true",
      "sonar.tests" => ".sonar-test-scope",
      "sonar.filesize.limit" => "100",
      "sonar.javascript.maxFileSize" => "100000",
      "sonar.javascript.detectBundles" => "false",
      "sonar.sca.enabled" => "true",
      "sonar.sca.allowManifestFailures" => "false",
      "sonar.sca.goNoResolve" => "false",
      "sonar.sca.mavenNoResolve" => "false",
      "sonar.sca.gradleNoResolve" => "false",
      "sonar.sca.pythonNoResolve" => "false",
      "sonar.sca.npmNoResolve" => "false",
      "sonar.sca.nugetNoResolve" => "false",
      "sonar.sca.cfamily" => "true",
      "sonar.rust.clippy.enabled" => "true",
      "sonar.rust.lcov.reportPaths" => "coverage/lcov.info",
      "sonar.javascript.lcov.reportPaths" => "coverage/js-lcov.info",
      "sonar.coverageReportPaths" => "coverage/script-coverage.xml",
      "sonar.cfamily.compile-commands" => "coverage/compile_commands.json",
      "sonar.cfamily.llvm-cov.reportPath" => "coverage/llvm-cov.txt",
      "sonar.newCode.referenceBranch" => "main",
      "sonar.qualitygate.wait" => "true",
      "sonar.qualitygate.timeout" => "600"
    }.freeze
    SBOM_PROPERTY = "sonar.sca.sbomImportPaths"
    SBOM_PATH = "release/media-compliance/media-runtime-inventory.spdx.json"

    def initialize(inputs, diagnostics)
      @inputs = inputs
      @diagnostics = diagnostics
    end

    def validate
      entries = @inputs.sonar_entries
      properties = entries.to_h { |entry| [entry.key, entry] }
      allowed_keys = BASE_KEYS.dup
      required_values = REQUIRED_VALUES.dup
      inventory_present = File.file?(@inputs.path(SBOM_PATH))
      if inventory_present
        allowed_keys.add(SBOM_PROPERTY)
        required_values[SBOM_PROPERTY] = SBOM_PATH
      end

      actual_keys = entries.map(&:key).to_set
      (actual_keys - allowed_keys).sort.each { |key| error("unrecognized Sonar property #{key.inspect}") }
      (allowed_keys - actual_keys).sort.each { |key| error("required Sonar property #{key.inspect} is missing") }
      EMPTY_KEYS.each do |key|
        entry = properties[key]
        error("#{key} must remain explicitly empty") if entry && !entry.value.empty?
      end
      required_values.each do |key, expected|
        entry = properties[key]
        error("#{key} must equal #{expected.inspect}") if entry && entry.value != expected
      end
      if !inventory_present && properties.key?(SBOM_PROPERTY)
        error("#{SBOM_PROPERTY} requires its committed matching SPDX inventory")
      end

      validate_source_inventory(properties["sonar.sources"]&.value)
      validate_sentinel
    end

    private

    def validate_source_inventory(configured)
      actual = configured.to_s.split(",")
      return if actual == @inputs.source_inventory

      error(
        "sonar.sources must exactly enumerate authored top-level entries: " \
        "expected #{@inputs.source_inventory.join(',')}"
      )
    end

    def validate_sentinel
      sentinel = @inputs.path(".sonar-test-scope")
      children = Dir.exist?(sentinel) ? Dir.children(sentinel).sort : []
      valid = children == [".gitkeep"] && File.zero?(File.join(sentinel, ".gitkeep"))
      error(".sonar-test-scope must contain only an empty .gitkeep") unless valid
    end

    def error(message)
      @diagnostics.add(message)
    end
  end
end
