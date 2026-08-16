# frozen_string_literal: true

require "json"
require "psych"

module WorkflowGuardrails
  class GuardrailError < StandardError; end

  class Diagnostics
    def initialize
      @errors = []
    end

    attr_reader :errors

    def add(message)
      @errors << message
    end

    def capture
      yield
    rescue GuardrailError, JSON::ParserError, KeyError, Psych::SyntaxError => e
      add(e.message)
      nil
    end

    def finish
      @errors.each { |error| warn "Workflow guardrail failed: #{error}" }
      @errors.empty? ? 0 : 1
    end
  end
end
