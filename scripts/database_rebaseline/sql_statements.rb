# frozen_string_literal: true

module RevaerDatabaseRebaseline
  class SqlStatements
    Boundary = Data.define(:ordinal, :byte_count, :line_number)

    attr_reader :boundaries

    def initialize(source)
      @source = source.b
      @boundaries = parse
    end

    def count
      boundaries.length
    end

    def complete_prefix?(byte_count)
      boundaries.any? { |boundary| boundary.byte_count == byte_count }
    end

    private

    def parse
      boundaries = []
      state = :normal
      dollar_tag = nil
      block_depth = 0
      line_number = 1
      index = 0

      while index < @source.bytesize
        current = @source.getbyte(index)
        following = @source.getbyte(index + 1)

        case state
        when :normal
          state, dollar_tag, block_depth, index = normal_transition(
            current, following, index, boundaries, line_number
          )
        when :single_quote
          state, index = quoted_transition(current, following, index, 39, :single_quote)
        when :double_quote
          state, index = quoted_transition(current, following, index, 34, :double_quote)
        when :line_comment
          state = :normal if current == 10
        when :block_comment
          state, block_depth, index = block_comment_transition(
            current, following, state, block_depth, index
          )
        when :dollar_quote
          if @source.byteslice(index, dollar_tag.bytesize) == dollar_tag
            index += dollar_tag.bytesize - 1
            state = :normal
            dollar_tag = nil
          end
        end

        line_number += 1 if current == 10
        index += 1
      end

      unless %i[normal line_comment].include?(state)
        raise Failure, "SQL candidate ends inside #{state.to_s.tr('_', ' ')}"
      end
      raise Failure, "SQL candidate contains no complete statements" if boundaries.empty?

      boundaries.freeze
    end

    def normal_transition(current, following, index, boundaries, line_number)
      case [current, following]
      when [45, 45]
        [:line_comment, nil, 0, index + 1]
      when [47, 42]
        [:block_comment, nil, 1, index + 1]
      else
        normal_token_transition(current, index, boundaries, line_number)
      end
    end

    def normal_token_transition(current, index, boundaries, line_number)
      return [:single_quote, nil, 0, index] if current == 39
      return [:double_quote, nil, 0, index] if current == 34

      if current == 36 && (tag = dollar_tag_at(index))
        return [:dollar_quote, tag, 0, index + tag.bytesize - 1]
      end

      if current == 59
        boundaries << Boundary.new(
          ordinal: boundaries.length + 1,
          byte_count: line_aligned_boundary(index),
          line_number:
        )
      end
      [:normal, nil, 0, index]
    end

    def quoted_transition(current, following, index, quote_byte, state)
      return [state, index + 1] if current == quote_byte && following == quote_byte
      return [state, index + 1] if current == 92 && !following.nil?
      return [:normal, index] if current == quote_byte

      [state, index]
    end

    def block_comment_transition(current, following, state, depth, index)
      if current == 47 && following == 42
        [state, depth + 1, index + 1]
      elsif current == 42 && following == 47
        next_depth = depth - 1
        [next_depth.zero? ? :normal : state, next_depth, index + 1]
      else
        [state, depth, index]
      end
    end

    def dollar_tag_at(index)
      tail = @source.byteslice(index, @source.bytesize - index)
      tail[/\A\$(?:[A-Za-z_][A-Za-z0-9_]*)?\$/]
    end

    def line_aligned_boundary(semicolon_index)
      newline_index = @source.index("\n", semicolon_index)
      return semicolon_index + 1 unless newline_index

      trailing = @source.byteslice(semicolon_index + 1, newline_index - semicolon_index - 1)
      trailing.match?(/\A[\t\r ]*\z/) ? newline_index + 1 : semicolon_index + 1
    end
  end
end
