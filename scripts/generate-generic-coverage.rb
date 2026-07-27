#!/usr/bin/env ruby
# frozen_string_literal: true

require "json"
require "pathname"
require "rexml/document"

LineCoverage = Struct.new(:hits, :branches, keyword_init: true)

class GenericCoverage
  def initialize(repository_root)
    @repository_root = Pathname.new(repository_root).expand_path
    @files = Hash.new { |files, path| files[path] = {} }
  end

  def add_line(path, line_number, hits)
    relative_path = normalize_path(path)
    line = @files[relative_path][line_number] ||= LineCoverage.new(hits: 0, branches: {})
    line.hits = [line.hits, hits].max
  end

  def add_branch(path, line_number, key, hits)
    relative_path = normalize_path(path)
    line = @files[relative_path][line_number] ||= LineCoverage.new(hits: 0, branches: {})
    line.branches[key] = [line.branches.fetch(key, 0), hits].max
  end

  def write(output_path)
    raise "script coverage did not contain any authored files" if @files.empty?

    document = REXML::Document.new
    document << REXML::XMLDecl.new("1.0", "UTF-8")
    root = document.add_element("coverage", { "version" => "1" })
    @files.sort.each do |path, lines|
      file = root.add_element("file", { "path" => path })
      lines.sort.each do |line_number, coverage|
        attributes = {
          "lineNumber" => line_number.to_s,
          "covered" => (coverage.hits.positive?).to_s
        }
        unless coverage.branches.empty?
          attributes["branchesToCover"] = coverage.branches.length.to_s
          attributes["coveredBranches"] = coverage.branches.values.count(&:positive?).to_s
        end
        file.add_element("lineToCover", attributes)
      end
    end
    File.open(output_path, "w:UTF-8") do |output|
      formatter = REXML::Formatters::Pretty.new(2)
      formatter.compact = true
      formatter.write(document, output)
      output.write("\n")
    end
    REXML::Document.new(File.read(output_path, encoding: "UTF-8"))
  end

  private

  def normalize_path(path)
    expanded_path = Pathname.new(path).expand_path
    relative_path = expanded_path.relative_path_from(@repository_root).to_s
    raise "coverage path escapes the repository: #{path}" if relative_path.start_with?("../")
    unless relative_path.match?(/\Ascripts\/.+\.(?:rb|sh)\z/)
      raise "coverage path is not an authored script: #{path}"
    end

    relative_path
  end
end

def kcov_report_path(directory)
  reports = Dir.glob(File.join(directory, "**", "sonarqube.xml")).map { |path| File.realpath(path) }.uniq
  raise "kcov did not produce exactly one report" unless reports.length == 1

  reports.fetch(0)
end

def import_kcov(coverage, directory)
  report = REXML::Document.new(File.read(kcov_report_path(directory), encoding: "UTF-8"))
  REXML::XPath.each(report, "/coverage/file") do |file|
    path = file.attributes.fetch("path").to_s
    REXML::XPath.each(file, "lineToCover") do |line|
      hits = line.attributes.fetch("covered").to_s == "true" ? 1 : 0
      coverage.add_line(path, Integer(line.attributes.fetch("lineNumber").to_s), hits)
    end
  end
end

def import_ruby(coverage, directory, repository_root)
  reports = Dir.glob(File.join(directory, "ruby-*.json")).sort
  raise "Ruby coverage did not produce any reports" if reports.empty?

  reports.each do |path|
    JSON.parse(File.read(path, encoding: "UTF-8"), symbolize_names: true).fetch(:files).each do |file|
      source_path = File.expand_path(file.fetch(:path), repository_root)
      file.fetch(:lines).each_with_index do |hits, index|
        coverage.add_line(source_path, index + 1, hits) if hits
      end
      file.fetch(:branches).each do |branch|
        coverage.add_branch(source_path, branch.fetch(:line), branch.fetch(:key), branch.fetch(:hits))
      end
    end
  end
end

unless ARGV.length == 3
  raise ArgumentError, "usage: #{$PROGRAM_NAME} KCOV_DIRECTORY RUBY_DIRECTORY OUTPUT_PATH"
end

repository_root = Dir.pwd
coverage = GenericCoverage.new(repository_root)
import_kcov(coverage, ARGV.fetch(0))
import_ruby(coverage, ARGV.fetch(1), repository_root)
coverage.write(ARGV.fetch(2))
