# frozen_string_literal: true

require "json"
require "open3"
require "psych"
require "set"
require "workflow_guardrails/diagnostics"

module WorkflowGuardrails
  class StrictYaml
    def initialize(path)
      @path = path
    end

    def load
      stream = Psych.parse_stream(File.read(@path, encoding: "UTF-8"), filename: @path)
      raise GuardrailError, "#{@path}: expected one YAML document" unless stream.children.length == 1

      root = stream.children.fetch(0).root
      raise GuardrailError, "#{@path}: YAML document must not be empty" unless root

      convert(root, "$")
    rescue Psych::SyntaxError => e
      raise GuardrailError, "#{@path}: invalid YAML: #{e.message}"
    end

    private

    def convert(node, location)
      case node
      when Psych::Nodes::Mapping
        convert_mapping(node, location)
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

    def convert_mapping(node, location)
      result = {}
      node.children.each_slice(2) do |key_node, value_node|
        unless key_node.is_a?(Psych::Nodes::Scalar)
          raise GuardrailError, "#{@path}:#{location}: mapping keys must be scalars"
        end
        key = key_node.value
        raise GuardrailError, "#{@path}:#{location}: duplicate YAML key #{key.inspect}" if result.key?(key)

        result[key] = convert(value_node, "#{location}.#{key}")
      end
      result
    end
  end

  class JavaProperties
    Entry = Struct.new(:key, :value, :line, keyword_init: true)

    def initialize(path)
      @path = path
      @errors = []
    end

    attr_reader :errors

    def parse
      entries = []
      seen = {}
      File.readlines(@path, chomp: true, encoding: "UTF-8").each_with_index do |text, index|
        line = index + 1
        stripped = text.lstrip
        next if stripped.empty? || stripped.start_with?("#", "!")

        @errors << "#{@path}:#{line}: leading property whitespace is forbidden" unless stripped == text
        @errors << "#{@path}:#{line}: property continuations are forbidden" if text[/\\+\z/].to_s.length.odd?
        separator = text.index(/[=: \t\f]/)
        key = separator ? text[0...separator] : text
        value = separator ? text[(separator + 1)..].to_s.sub(/\A[ \t\f=:]*/, "") : ""
        @errors << "#{@path}:#{line}: escaped property keys are forbidden" if key.include?("\\")
        if seen.key?(key)
          @errors << "#{@path}:#{line}: duplicate property #{key.inspect}; first defined on line #{seen.fetch(key)}"
        else
          seen[key] = line
        end
        entries << Entry.new(key:, value:, line:)
      end
      entries
    end
  end

  class InputLoader
    JUST_MODULES = %w[quality database ui media release images docs].map do |name|
      "just/#{name}.just"
    end.freeze

    def initialize(root, diagnostics)
      @root = File.expand_path(root)
      @diagnostics = diagnostics
      @workflow_root = File.join(@root, ".github", "workflows")
      @action_root = File.join(@root, ".github", "actions")
      @documents = {}
      @recipes = Set.new
      @recipe_owners = {}
      @sonar_entries = []
      @required_contexts = []
      @image_matrix = {}
      @source_inventory = []
    end

    attr_reader :action_root, :documents, :image_matrix, :recipe_owners, :recipes,
                :required_contexts, :root, :sonar_entries, :source_inventory,
                :workflow_root

    def load
      validate_just_index
      load_recipes(File.join(@root, "justfile"), Set.new)
      load_yaml_documents
      load_sonar_properties
      load_required_contexts
      load_image_matrix
      load_source_inventory
      self
    end

    def path(relative)
      File.join(@root, relative)
    end

    def document(relative)
      @documents[path(relative)]
    end

    private

    def validate_just_index
      index_path = path("justfile")
      statements = File.readlines(index_path, chomp: true, encoding: "UTF-8")
        .map(&:strip)
        .reject { |line| line.empty? || line.start_with?("#") }
      expected = [
        'set shell := ["bash", "-c"]',
        *JUST_MODULES.map { |module_path| "import '#{module_path}'" }
      ]
      unless statements == expected
        @diagnostics.add("justfile must remain the exact shared shell setting and seven approved imports")
      end

      module_files = Dir.glob(path("just/*.just")).map { |entry| relative(entry) }.sort
      expected_modules = JUST_MODULES.sort
      return if module_files == expected_modules

      @diagnostics.add("just/ must contain exactly the seven approved ADR 482 modules")
    end

    def load_recipes(file_path, visited)
      expanded = File.expand_path(file_path)
      if visited.include?(expanded)
        @diagnostics.add("Just import cycle detected at #{relative(expanded)}")
        return
      end
      unless File.file?(expanded)
        @diagnostics.add("Just import is missing: #{relative(expanded)}")
        return
      end

      visited.add(expanded)
      File.readlines(expanded, encoding: "UTF-8").each do |line|
        imported = line[/\A\s*\??import\s+['"]([^'"]+)['"]\s*\z/, 1]
        if imported
          load_recipes(File.join(File.dirname(expanded), imported), visited)
          next
        end
        next if line.match?(/\A\s*(?:#|set\s|$)/)

        recipe = line[/\A([a-zA-Z0-9_-]+)(?:\s+[^:]*)?:/, 1]
        next unless recipe

        if @recipe_owners.key?(recipe)
          @diagnostics.add(
            "Just recipe #{recipe.inspect} has multiple owners: " \
            "#{@recipe_owners.fetch(recipe)} and #{relative(expanded)}"
          )
        else
          @recipe_owners[recipe] = relative(expanded)
          @recipes.add(recipe)
        end
      end
      visited.delete(expanded)
    end

    def load_yaml_documents
      yaml_paths = [@workflow_root, @action_root].flat_map do |directory|
        Dir.glob(File.join(directory, "**", "*.{yml,yaml}"))
      end.sort
      yaml_paths.each do |yaml_path|
        document = @diagnostics.capture { StrictYaml.new(yaml_path).load }
        next unless document

        unless document.is_a?(Hash)
          @diagnostics.add("#{yaml_path}: root must be a mapping")
          next
        end
        @documents[yaml_path] = document
      end
    end

    def load_sonar_properties
      parser = JavaProperties.new(path("sonar-project.properties"))
      @sonar_entries = parser.parse
      parser.errors.each { |error| @diagnostics.add(error) }
    end

    def load_required_contexts
      required_path = path("config/required-pr-checks.txt")
      @required_contexts = File.readlines(required_path, chomp: true, encoding: "UTF-8")
        .map(&:strip)
        .reject { |line| line.empty? || line.start_with?("#") }
    rescue Errno::ENOENT => e
      @diagnostics.add(e.message)
    end

    def load_image_matrix
      matrix_path = path(".github/matrices/build-images.json")
      @image_matrix = @diagnostics.capture do
        JSON.parse(File.read(matrix_path, encoding: "UTF-8"))
      end || {}
    rescue Errno::ENOENT => e
      @diagnostics.add(e.message)
    end

    def load_source_inventory
      stdout, stderr, status = Open3.capture3(
        "git", "-C", @root, "ls-files", "--cached", "--others", "--exclude-standard", "-z"
      )
      unless status.success?
        @diagnostics.add("unable to enumerate authored Sonar sources: #{stderr.strip}")
        return
      end
      @source_inventory = stdout.split("\0").reject(&:empty?)
        .map { |entry| entry.split("/", 2).first }.uniq.sort
      @source_inventory.delete(".sonar-test-scope")
    end

    def relative(path)
      path.delete_prefix("#{@root}/")
    end
  end
end
