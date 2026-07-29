#!/usr/bin/env ruby
# frozen_string_literal: true

require "optparse"
require "psych"
require "set"

class GuardrailError < StandardError; end

class StrictYaml
  def initialize(path)
    @path = path
  end

  def load
    stream = Psych.parse_stream(File.read(@path, encoding: "UTF-8"), filename: @path)
    raise GuardrailError, "#{@path}: expected one YAML document" unless stream.children.length == 1

    convert(stream.children.fetch(0).root, "$")
  rescue Psych::SyntaxError => e
    raise GuardrailError, "#{@path}: invalid YAML: #{e.message}"
  end

  private

  def convert(node, location)
    case node
    when Psych::Nodes::Mapping
      result = {}
      node.children.each_slice(2) do |key_node, value_node|
        raise GuardrailError, "#{@path}:#{location}: mapping keys must be scalars" unless key_node.is_a?(Psych::Nodes::Scalar)

        key = key_node.value
        raise GuardrailError, "#{@path}:#{location}: duplicate YAML key #{key.inspect}" if result.key?(key)

        result[key] = convert(value_node, "#{location}.#{key}")
      end
      result
    when Psych::Nodes::Sequence
      node.children.each_with_index.map { |child, index| convert(child, "#{location}[#{index}]") }
    when Psych::Nodes::Scalar
      node.value
    when Psych::Nodes::Alias
      raise GuardrailError, "#{@path}:#{location}: YAML aliases are forbidden"
    else
      raise GuardrailError, "#{@path}:#{location}: unsupported YAML node #{node.class}"
    end
  end
end

class JavaProperties
  Entry = Data.define(:key, :value, :line)
  PROPERTY_WHITESPACE = /[ \t\f]/

  def initialize(path)
    @path = path
    @errors = []
  end

  attr_reader :errors

  def parse
    entries = []
    seen = {}
    logical_lines.each do |text, line_number, continued|
      stripped = text.lstrip
      next if stripped.empty? || stripped.start_with?("#", "!")

      @errors << "#{@path}:#{line_number}: leading property whitespace is forbidden" unless stripped == text
      @errors << "#{@path}:#{line_number}: property continuations are forbidden" if continued
      raw_key, raw_value = split_property(text)
      @errors << "#{@path}:#{line_number}: escaped property keys are forbidden" if raw_key.include?("\\")
      key = decode(raw_key, line_number)
      value = decode(raw_value, line_number)
      if seen.key?(key)
        @errors << "#{@path}:#{line_number}: duplicate logical property #{key.inspect}; first defined on line #{seen.fetch(key)}"
      else
        seen[key] = line_number
      end
      entries << Entry.new(key:, value:, line: line_number)
    end
    entries
  end

  private

  def logical_lines
    lines = File.readlines(@path, chomp: true, encoding: "UTF-8")
    result = []
    index = 0
    while index < lines.length
      first_line = index + 1
      text = lines.fetch(index).delete_suffix("\r")
      continued = false
      while text[/\\+\z/].to_s.length.odd?
        continued = true
        text = text[0...-1]
        index += 1
        break if index >= lines.length

        text += lines.fetch(index).delete_suffix("\r").sub(/\A[ \t\f]*/, "")
      end
      result << [text, first_line, continued]
      index += 1
    end
    result
  end

  def split_property(text)
    start = text.index(/[^ \t\f]/) || text.length
    escaped = false
    separator = nil
    (start...text.length).each do |index|
      char = text[index]
      if escaped
        escaped = false
      elsif char == "\\"
        escaped = true
      elsif char == "=" || char == ":" || char.match?(PROPERTY_WHITESPACE)
        separator = index
        break
      end
    end
    return [text[start..] || "", ""] unless separator

    raw_key = text[start...separator]
    value_start = separator
    value_start += 1 while value_start < text.length && text[value_start].match?(PROPERTY_WHITESPACE)
    value_start += 1 if value_start < text.length && ["=", ":"].include?(text[value_start])
    value_start += 1 while value_start < text.length && text[value_start].match?(PROPERTY_WHITESPACE)
    [raw_key, text[value_start..] || ""]
  end

  def decode(text, line_number)
    output = +""
    index = 0
    while index < text.length
      unless text[index] == "\\"
        output << text[index]
        index += 1
        next
      end
      index += 1
      if index >= text.length
        @errors << "#{@path}:#{line_number}: dangling property escape"
        break
      end
      escaped = text[index]
      if escaped == "u"
        digits = text[(index + 1), 4]
        unless digits&.match?(/\A[0-9a-fA-F]{4}\z/)
          @errors << "#{@path}:#{line_number}: invalid Unicode property escape"
          index += 1
          next
        end
        output << [digits.to_i(16)].pack("U")
        index += 5
        next
      end
      output << { "t" => "\t", "n" => "\n", "r" => "\r", "f" => "\f" }.fetch(escaped, escaped)
      index += 1
    end
    output
  end
end

class WorkflowStructureGuardrails
  EXTERNAL_ACTION = %r{\A[^./][^/]*/[^@]+@[0-9a-f]{40}\z}
  IMAGE_INVENTORY_STEP = "Inventory digest-qualified image"
  IMAGE_SCAN_STEP = "Scan digest-qualified image"
  PERMISSION_VALUES = Set.new(%w[none read write]).freeze
  SONAR_KEYS = Set.new(%w[
    sonar.projectKey sonar.organization sonar.sourceEncoding
    sonar.scanner.excludeHiddenFiles sonar.text.activate
    sonar.text.inclusions.activate sonar.text.inclusions sonar.html.file.suffixes
    sonar.tsql.file.suffixes sonar.plsql.file.suffixes sonar.plsql.defaultSchema
    sonar.featureflag.cloud-security-enable-generic-yaml-and-json-analyzer
    sonar.yaml.activate sonar.json.activate sonar.lang.patterns.kubernetes
    sonar.lang.patterns.yaml sonar.sources sonar.scm.exclusions.disabled
    sonar.scm.disabled sonar.scm.provider sonar.scm.forceReloadAll
    sonar.sensor.cache.project.enable sonar.scanner.keepReport sonar.exclusions
    sonar.inclusions sonar.tests sonar.test.exclusions sonar.test.inclusions
    sonar.coverage.exclusions sonar.cpd.exclusions sonar.issue.ignore.multicriteria
    sonar.issue.ignore.allfile sonar.issue.ignore.block sonar.issue.enforce.multicriteria
    sonar.filesize.limit sonar.javascript.exclusions sonar.javascript.maxFileSize
    sonar.javascript.detectBundles sonar.sca.enabled sonar.sca.exclusions
    sonar.sca.allowManifestFailures sonar.sca.goNoResolve sonar.sca.mavenNoResolve
    sonar.sca.gradleNoResolve sonar.sca.pythonNoResolve sonar.sca.npmNoResolve
    sonar.sca.nugetNoResolve sonar.sca.cfamily sonar.sca.sbomImportPaths
    sonar.rust.clippy.enabled sonar.rust.lcov.reportPaths
    sonar.javascript.lcov.reportPaths sonar.coverageReportPaths sonar.cfamily.compile-commands
    sonar.cfamily.llvm-cov.reportPath
    sonar.newCode.referenceBranch sonar.qualitygate.wait sonar.qualitygate.timeout
  ]).freeze
  EMPTY_SONAR_KEYS = Set.new(%w[
    sonar.exclusions sonar.inclusions sonar.test.exclusions sonar.test.inclusions
    sonar.coverage.exclusions sonar.cpd.exclusions sonar.issue.ignore.multicriteria
    sonar.issue.ignore.allfile sonar.issue.ignore.block sonar.issue.enforce.multicriteria
    sonar.javascript.exclusions sonar.sca.exclusions
  ]).freeze

  def initialize(options)
    @options = options
    @errors = []
    @recipes = Set.new
    load_recipes(options.fetch(:justfile), Set.new)
  end

  def run
    yaml_paths.each { |path| validate_yaml(path) }
    validate_sonar
    @errors.each { |error| warn "Workflow guardrail failed: #{error}" }
    @errors.empty? ? 0 : 1
  end

  private

  def load_recipes(path, visited)
    expanded_path = File.expand_path(path)
    return if visited.include?(expanded_path)

    visited.add(expanded_path)
    File.readlines(expanded_path, encoding: "UTF-8").each do |line|
      recipe = line[/\A([a-zA-Z0-9_-]+)(?:\s+[^:]*)?:/, 1]
      @recipes.add(recipe) if recipe
      imported_path = line[/\A\??import\s+['"]([^'"]+)['"]\s*\z/, 1]
      load_recipes(File.join(File.dirname(expanded_path), imported_path), visited) if imported_path
    end
  end

  def yaml_paths
    @options.values_at(:workflows, :actions).flat_map { |root| Dir.glob(File.join(root, "**", "*.{yml,yaml}")) }.sort
  end

  def validate_yaml(path)
    document = StrictYaml.new(path).load
    raise GuardrailError, "#{path}: root must be a mapping" unless document.is_a?(Hash)

    path.start_with?(@options.fetch(:workflows)) ? validate_workflow(path, document) : validate_action(path, document)
  rescue GuardrailError => e
    @errors << e.message
  end

  def validate_workflow(path, document)
    jobs = document["jobs"]
    raise GuardrailError, "#{path}: jobs must be a non-empty mapping" unless jobs.is_a?(Hash) && !jobs.empty?

    validate_permissions(path, "workflow", document["permissions"]) if document.key?("permissions")
    jobs.each do |job_name, job|
      unless job.is_a?(Hash)
        @errors << "#{path}: job #{job_name} must be a mapping"
        next
      end
      validate_condition(path, "job #{job_name}", job["if"]) if job.key?("if")
      validate_permissions(path, "job #{job_name}", job["permissions"]) if job.key?("permissions")
      validate_postgres_service(path, job_name, job)
      if job.key?("uses")
        @errors << "#{path}: reusable job #{job_name} must not define steps" if job.key?("steps")
        validate_uses(path, "job #{job_name}", job["uses"])
      else
        validate_steps(path, "job #{job_name}", job["steps"])
      end
    end
    validate_build_images(path, jobs) if File.basename(path) == "build-images.yml"
  end

  def validate_action(path, document)
    runs = document["runs"]
    validate_steps(path, "composite action", runs["steps"]) if runs.is_a?(Hash) && runs["using"] == "composite"
  end

  def validate_postgres_service(path, job_name, job)
    postgres = job.dig("services", "postgres")
    return unless postgres.is_a?(Hash)

    service_env = postgres["env"]
    job_env = job["env"]
    unless service_env.is_a?(Hash) && job_env.is_a?(Hash)
      @errors << "#{path}: job #{job_name} Postgres service and job must define environment mappings"
      return
    end
    user = service_env["POSTGRES_USER"]
    password = service_env["POSTGRES_PASSWORD"]
    database = service_env["POSTGRES_DB"]
    unless [user, password, database].all? { |value| value.is_a?(String) && !value.empty? }
      @errors << "#{path}: job #{job_name} Postgres service must define user, password, and database"
      return
    end

    expected_url = "postgres://#{user}:#{password}@localhost:5432/#{database}"
    %w[REVAER_TEST_DATABASE_URL DATABASE_URL].each do |key|
      unless job_env[key] == expected_url
        @errors << "#{path}: job #{job_name} #{key} must exactly match its Postgres service credential"
      end
    end
    expected_admin_url = "postgres://#{user}:#{password}@localhost:5432/postgres"
    if job_env.key?("E2E_DB_ADMIN_URL") && job_env["E2E_DB_ADMIN_URL"] != expected_admin_url
      @errors << "#{path}: job #{job_name} E2E_DB_ADMIN_URL must exactly match its Postgres service credential"
    end
  end

  def validate_steps(path, owner, steps)
    unless steps.is_a?(Array) && !steps.empty?
      @errors << "#{path}: #{owner} steps must be a non-empty sequence"
      return
    end
    steps.each_with_index do |step, index|
      label = "#{owner} step #{index + 1}"
      unless step.is_a?(Hash)
        @errors << "#{path}: #{label} must be a mapping"
        next
      end
      command_keys = %w[uses run].select { |key| step.key?(key) }
      @errors << "#{path}: #{label} must define exactly one of uses or run" unless command_keys.length == 1
      validate_uses(path, label, step["uses"]) if step.key?("uses")
      validate_run(path, label, step["run"]) if step.key?("run")
      validate_condition(path, label, step["if"]) if step.key?("if")
    end
  end

  def validate_uses(path, owner, uses)
    return if uses.is_a?(String) && (uses.start_with?("./", "docker://") || uses.match?(EXTERNAL_ACTION))

    @errors << "#{path}: #{owner} external action must use a full lowercase 40-character commit SHA: #{uses.inspect}"
  end

  def validate_run(path, owner, run)
    unless run.is_a?(String) && !run.empty?
      @errors << "#{path}: #{owner} run must be a non-empty scalar"
      return
    end
    @errors << "#{path}: #{owner} must not interpolate inputs directly in run" if run.match?(/\$\{\{\s*inputs\./)
    run.scan(/(?:\A|[\n;&|()]\s*)just\s+([a-zA-Z0-9_-]+)/).flatten.each do |recipe|
      @errors << "#{path}: #{owner} calls unknown just recipe #{recipe.inspect}" unless @recipes.include?(recipe)
    end
    if run.match?(/(?:\A|[\n;&|()]\s*)cargo\s+(?:build|check|clippy|fmt|test)\b/)
      @errors << "#{path}: #{owner} must run build, check, lint, format, and test gates through just"
    end
  end

  def validate_condition(path, owner, condition)
    unless condition.is_a?(String) && !condition.strip.empty? && !condition.include?("\n")
      @errors << "#{path}: #{owner} if condition must be one non-empty scalar"
    end
  end

  def validate_permissions(path, owner, permissions)
    unless permissions.is_a?(Hash) && !permissions.empty?
      @errors << "#{path}: #{owner} permissions must be a non-empty mapping"
      return
    end
    permissions.each do |scope, value|
      unless scope.match?(/\A[a-z-]+\z/) && PERMISSION_VALUES.include?(value)
        @errors << "#{path}: #{owner} permission #{scope.inspect} must be none, read, or write"
      end
    end
  end

  def validate_build_images(path, jobs)
    build = jobs["build"]
    unless build.is_a?(Hash) && build["steps"].is_a?(Array)
      @errors << "#{path}: build job with semantic steps is required"
      return
    end
    steps = build.fetch("steps")
    by_name = named_steps(steps)
    return unless validate_image_step_order(path, steps, by_name)

    validate_image_digest_binding(path, by_name)
    validate_image_scan(path, by_name)
    validate_image_evidence(path, by_name)
  end

  def named_steps(steps)
    steps.filter_map { |step| [step["name"], step] if step.is_a?(Hash) && step["name"] }.to_h
  end

  def validate_image_step_order(path, steps, by_name)
    ordered_names = [
      "Build & Push Image",
      "Validate supplied compliance authorization against built digest",
      IMAGE_INVENTORY_STEP,
      IMAGE_SCAN_STEP,
      "Verify HIGH and CRITICAL findings",
      "Upload scan results",
      "Generate digest-bound compliance evidence",
      "Validate digest-bound compliance evidence",
      "Sign image and compliance evidence",
      "Verify signed compliance evidence"
    ]
    positions = ordered_names.map { |name| steps.index(by_name[name]) }
    if positions.any?(&:nil?) || positions != positions.sort
      @errors << "#{path}: publication must build, resolve, authorize, inventory, scan, retain SARIF, generate, validate, sign, and verify evidence in order"
      return false
    end
    true
  end

  def validate_image_digest_binding(path, by_name)
    built_ref = "${{ steps.build_image.outputs.image_reference }}"
    build_run = by_name.fetch("Build & Push Image")["run"].to_s
    unless build_run.include?("--metadata-file") && build_run.include?("imagetools inspect") && build_run.include?("image_reference=")
      @errors << "#{path}: image build must emit and independently resolve an immutable OCI digest"
    end
    supplied = by_name.fetch("Validate supplied compliance authorization against built digest")
    unless supplied.dig("env", "IMAGE_REFERENCE") == built_ref && supplied["run"].to_s.include?("scripts/validate-final-image-compliance-bundle.sh")
      @errors << "#{path}: supplied publication evidence must be validated against the built digest"
    end
    [IMAGE_INVENTORY_STEP, IMAGE_SCAN_STEP].each do |name|
      @errors << "#{path}: #{name} must inspect the built digest" unless by_name.fetch(name).dig("with", "image-ref") == built_ref
    end
  end

  def validate_image_scan(path, by_name)
    scan = by_name.fetch(IMAGE_SCAN_STEP)
    unless scan.dig("with", "severity") == "HIGH,CRITICAL" && scan.dig("with", "exit-code") == "0"
      @errors << "#{path}: Trivy must emit HIGH/CRITICAL SARIF for the explicit verifier"
    end
    verify = by_name.fetch("Verify HIGH and CRITICAL findings")["run"].to_s
    @errors << "#{path}: Trivy findings must be verified through just" unless verify.include?("just trivy-sarif-verify")
    category_if = by_name.fetch("Resolve code scanning category")["if"].to_s
    @errors << "#{path}: SARIF category resolution must survive a failed vulnerability gate" unless category_if.include?("always()")
    upload_if = by_name.fetch("Upload scan results")["if"].to_s
    @errors << "#{path}: Trivy SARIF must be retained with always()" unless upload_if.include?("always()") && upload_if.include?("trivy-results.sarif")
  end

  def validate_image_evidence(path, by_name)
    built_ref = "${{ steps.build_image.outputs.image_reference }}"
    {
      "Generate digest-bound compliance evidence" => "just image-compliance-generate",
      "Validate digest-bound compliance evidence" => "just image-compliance-validate",
      "Sign image and compliance evidence" => "cosign attest",
      "Verify signed compliance evidence" => "cosign verify-attestation"
    }.each do |name, command|
      step = by_name.fetch(name)
      unless step.dig("env", "IMAGE_REFERENCE") == built_ref && step["run"].to_s.include?(command)
        @errors << "#{path}: #{name} must consume the built digest and run #{command}"
      end
    end
  end

  def validate_sonar
    parser = JavaProperties.new(@options.fetch(:sonar))
    entries = parser.parse
    @errors.concat(parser.errors)
    actual = entries.map(&:key)
    (actual.to_set - SONAR_KEYS).each { |key| @errors << "unrecognized Sonar property #{key.inspect}" }
    (SONAR_KEYS - actual.to_set).each { |key| @errors << "required Sonar property #{key.inspect} is missing" }
    entries.each do |entry|
      if EMPTY_SONAR_KEYS.include?(entry.key) && !entry.value.empty?
        @errors << "#{@options.fetch(:sonar)}:#{entry.line}: #{entry.key} must remain explicitly empty"
      end
      if entry.key.match?(/(?:enabled|activate)\z/) && entry.value == "false"
        @errors << "#{@options.fetch(:sonar)}:#{entry.line}: analyzer activation must not be disabled"
      end
      if %w[sonar.skip sonar.scanner.skip sonar.scm.disabled].include?(entry.key) && entry.value == "true"
        @errors << "#{@options.fetch(:sonar)}:#{entry.line}: Sonar scanning and SCM analysis must remain enabled"
      end
    end
  end
end

options = {}
OptionParser.new do |parser|
  parser.on("--workflows PATH") { |value| options[:workflows] = value }
  parser.on("--actions PATH") { |value| options[:actions] = value }
  parser.on("--sonar PATH") { |value| options[:sonar] = value }
  parser.on("--justfile PATH") { |value| options[:justfile] = value }
end.parse!

missing = %i[workflows actions sonar justfile].reject { |key| options.key?(key) }
abort "missing required options: #{missing.join(', ')}" unless missing.empty?

exit WorkflowStructureGuardrails.new(options).run
