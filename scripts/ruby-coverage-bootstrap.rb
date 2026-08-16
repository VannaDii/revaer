# frozen_string_literal: true

require "coverage"

Coverage.start(lines: true, branches: true)
require_relative "ruby-coverage-hook"
