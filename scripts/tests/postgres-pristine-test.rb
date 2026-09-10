# frozen_string_literal: true

require_relative "../postgres_pristine/container"
require_relative "../postgres_pristine/snapshot"

module RevaerPostgresPristine
  class FocusedTests
    CASES = {
      "pg_namespace" => ["", "REVOKE USAGE ON SCHEMA public FROM PUBLIC"],
      "pg_extension" => ["", "UPDATE pg_extension SET extversion = 'fixture-version' WHERE extname = 'plpgsql'"],
      "pg_class" => ["CREATE TABLE public.fixture_table (id integer)", "ALTER TABLE public.fixture_table ENABLE ROW LEVEL SECURITY"],
      "pg_proc" => ["", "ALTER FUNCTION pg_catalog.abs(integer) SECURITY DEFINER"],
      "pg_type" => ["", "CREATE TYPE public.fixture_type AS ENUM ('first', 'second')"],
      "pg_trigger" => ["CREATE TABLE public.fixture_table (id integer); CREATE FUNCTION public.fixture_trigger() RETURNS trigger LANGUAGE plpgsql AS 'BEGIN RETURN NEW; END'", "CREATE TRIGGER fixture_trigger BEFORE INSERT ON public.fixture_table FOR EACH ROW EXECUTE FUNCTION public.fixture_trigger()"],
      "pg_event_trigger" => ["CREATE FUNCTION public.fixture_event() RETURNS event_trigger LANGUAGE plpgsql AS 'BEGIN END'", "CREATE EVENT TRIGGER fixture_event ON ddl_command_end EXECUTE FUNCTION public.fixture_event()"],
      "pg_language" => ["", "REVOKE USAGE ON LANGUAGE plpgsql FROM PUBLIC"],
      "pg_cast" => ["", "UPDATE pg_cast SET castcontext = 'e' WHERE castsource = 'integer'::regtype AND casttarget = 'bigint'::regtype"],
      "pg_collation" => ["", "CREATE COLLATION public.fixture_collation (provider = libc, locale = 'C')"],
      "pg_conversion" => ["", "CREATE CONVERSION public.fixture_conversion FOR 'UTF8' TO 'LATIN1' FROM pg_catalog.utf8_to_iso8859_1"],
      "pg_operator" => ["", "CREATE OPERATOR public.=== (LEFTARG = integer, RIGHTARG = integer, FUNCTION = pg_catalog.int4eq)"],
      "pg_opclass" => ["", "CREATE OPERATOR CLASS public.fixture_class FOR TYPE int4 USING btree AS OPERATOR 1 < (int4, int4), FUNCTION 1 pg_catalog.btint4cmp(int4, int4)"],
      "pg_opfamily" => ["", "CREATE OPERATOR FAMILY public.fixture_family USING btree"],
      "pg_amop" => ["CREATE OPERATOR FAMILY public.fixture_family USING btree", "ALTER OPERATOR FAMILY public.fixture_family USING btree ADD OPERATOR 1 < (int4, int4)"],
      "pg_amproc" => ["CREATE OPERATOR FAMILY public.fixture_family USING btree", "ALTER OPERATOR FAMILY public.fixture_family USING btree ADD FUNCTION 1 (int4, int4) pg_catalog.btint4cmp(int4, int4)"],
      "pg_ts_config" => ["", "CREATE TEXT SEARCH CONFIGURATION public.fixture_config (COPY = pg_catalog.simple)"],
      "pg_ts_dict" => ["", "CREATE TEXT SEARCH DICTIONARY public.fixture_dict (TEMPLATE = pg_catalog.simple)"],
      "pg_ts_parser" => ["", "CREATE TEXT SEARCH PARSER public.fixture_parser (START = pg_catalog.prsd_start, GETTOKEN = pg_catalog.prsd_nexttoken, END = pg_catalog.prsd_end, LEXTYPES = pg_catalog.prsd_lextype, HEADLINE = pg_catalog.prsd_headline)"],
      "pg_ts_template" => ["", "CREATE TEXT SEARCH TEMPLATE public.fixture_template (INIT = pg_catalog.dsimple_init, LEXIZE = pg_catalog.dsimple_lexize)"],
      "pg_foreign_data_wrapper" => ["", "CREATE FOREIGN DATA WRAPPER fixture_wrapper"],
      "pg_foreign_server" => ["CREATE FOREIGN DATA WRAPPER fixture_wrapper", "CREATE SERVER fixture_server FOREIGN DATA WRAPPER fixture_wrapper"],
      "pg_user_mapping" => ["CREATE FOREIGN DATA WRAPPER fixture_wrapper; CREATE SERVER fixture_server FOREIGN DATA WRAPPER fixture_wrapper", "CREATE USER MAPPING FOR CURRENT_USER SERVER fixture_server OPTIONS (user 'fixture')"],
      "pg_policy" => ["CREATE TABLE public.fixture_table (id integer)", "CREATE POLICY fixture_policy ON public.fixture_table FOR SELECT USING (id > 7)"],
      "pg_publication" => ["", "CREATE PUBLICATION fixture_publication WITH (publish = 'insert, update')"],
      "pg_publication_rel" => ["CREATE TABLE public.fixture_table (id integer); CREATE PUBLICATION fixture_publication", "ALTER PUBLICATION fixture_publication ADD TABLE public.fixture_table (id) WHERE (id > 7)"],
      "pg_subscription" => ["", "CREATE SUBSCRIPTION fixture_subscription CONNECTION 'dbname=unused' PUBLICATION fixture_publication WITH (connect = false, slot_name = NONE)"]
    }.freeze

    def initialize(root)
      @root = root
      @assertions = 0
    end

    def run
      Container.new(@root).with_running do |container|
        @container = container
        normalization_tests
        column_accounting_tests
        CASES.each { |catalog, (setup, mutation)| mutation_test(catalog, setup, mutation) }
        security_attribute_tests
      end
      puts "pristine focused validation: #{@assertions} assertions passed; all 27 catalog classes exercised"
    end

    private

    def assert(value, message)
      raise Failure, "focused validation: #{message}" unless value

      @assertions += 1
    end

    def rejected(message)
      begin
        yield
      rescue Failure => error
        assert(error.message.include?(message), "unexpected failure category")
        return
      end
      raise Failure, "focused validation: expected rejection did not occur"
    end

    def fresh
      database, owner = @container.provision(SecureRandom.hex(6))
      snapshot = Snapshot.new(@container, database, owner)
      [database, owner, snapshot]
    end

    def execute(database, statement)
      @container.sql!("#{statement};", database: database) unless statement.empty?
    end

    def rows(bytes, catalog)
      bytes.lines.select { |line| line.start_with?("#{catalog}\t") }
    end

    def mutation_test(catalog, setup, mutation)
      database, _owner, snapshot = fresh
      execute(database, setup)
      before = snapshot.read
      execute(database, mutation)
      if %w[pg_user_mapping pg_subscription].include?(catalog)
        rejected("SQLSTATE=42501") { snapshot.read }
      else
        after = snapshot.read
        assert(rows(before, catalog) != rows(after, catalog), "#{catalog} mutation was not captured")
      end
      puts "catalog mutation captured: #{catalog}"
    ensure
      execute(database, "DROP SUBSCRIPTION IF EXISTS fixture_subscription") if catalog == "pg_subscription" && database
    end

    def normalization_tests
      database, owner, snapshot = fresh
      baseline = snapshot.read
      _other_database, _other_owner, other = fresh
      assert(other.read == baseline, "database and owner identities did not normalize")
      assert(baseline.encoding == Encoding::UTF_8 && baseline.valid_encoding?, "snapshot is not UTF-8")
      assert(baseline.end_with?("\n") && !baseline.include?("\r"), "snapshot is not LF terminated")
      assert(baseline.lines == baseline.lines.sort_by(&:b), "snapshot is not bytewise sorted")
      assert(baseline.lines.grep(/^#catalog\t/).length == 27, "catalog inventory is incomplete")
      assert(!baseline.include?(database) && !baseline.include?(owner), "fixture identity was retained")
      assert(snapshot.read == baseline, "repeated read is unstable")
      execute(database, "UPDATE pg_class SET relpages = relpages + 100, reltuples = reltuples + 50, relallvisible = relallvisible + 5 WHERE oid = 'pg_catalog.pg_class'::regclass")
      assert(snapshot.read == baseline, "planner statistics affected evidence")
      temporary = "CREATE TEMPORARY TABLE fixture_temp (id integer, payload text);"
      assert(snapshot.read(prefix: temporary) == baseline, "temporary relation identity affected evidence")
      ddl = "CREATE TABLE public.fixture_toast (id integer, payload text); CREATE FUNCTION public.fixture_function(value integer DEFAULT 7) RETURNS integer LANGUAGE SQL RETURN value + 1"
      @container.sql!("#{ddl};", database: database, user: owner)
      initial = snapshot.read
      @container.sql!("DROP TABLE public.fixture_toast; DROP FUNCTION public.fixture_function(integer); #{ddl};", database: database, user: owner)
      assert(snapshot.read == initial, "reallocated OIDs affected evidence")
      assert(!initial.match?(/pg_toast_[0-9]+/), "TOAST identity contains an OID")
      rejected("owner identity") { Snapshot.new(@container, database, "postgres").read }
      path = File.join(@root, "target/postgres-pristine/postgres-pristine-16.14.tsv")
      assert(File.binread(path) == baseline, "generated evidence differs from independent fixture") if File.file?(path)
    end

    def column_accounting_tests
      _database, _owner, snapshot = fresh
      assert(CASES.keys.sort == COLUMNS.keys.sort, "mutation suite does not cover the exact catalog set")
      query = Query.new
      snapshot.inventory.each do |catalog, columns|
        projected = query.projection(catalog, columns)
        expected_count = columns.count { |name, _type| name != "oid" && !EXCLUDED.key?(name) }
        assert(projected.length == expected_count, "unaccounted catalog columns")
        rejected("inventory drift") { query.projection(catalog, columns + [["unexpected", "text"]]) }
        rejected("inventory drift") { query.projection(catalog, columns.drop(1)) }
      end
      rejected("column type") { query.expression("nspname", "unsupported_type") }
    end

    def security_attribute_tests
      setup = "CREATE TABLE public.fixture_table (id integer); " \
              "CREATE FUNCTION public.fixture_function(value integer DEFAULT 7) RETURNS integer LANGUAGE SQL RETURN value + 1; " \
              "CREATE TEXT SEARCH CONFIGURATION public.fixture_config (COPY = pg_catalog.simple)"
      cases = [
        ["pg_proc", "ALTER FUNCTION public.fixture_function(integer) SET search_path = public"],
        ["pg_proc", "ALTER FUNCTION public.fixture_function(integer) LEAKPROOF"],
        ["pg_proc", "REVOKE ALL ON FUNCTION public.fixture_function(integer) FROM PUBLIC"],
        ["pg_proc", "CREATE OR REPLACE FUNCTION public.fixture_function(value integer DEFAULT 9) RETURNS integer LANGUAGE SQL RETURN value + 2"],
        ["pg_class", "GRANT SELECT(id) ON public.fixture_table TO PUBLIC"],
        ["pg_class", "ALTER TABLE public.fixture_table ALTER COLUMN id SET NOT NULL"],
        ["pg_class", "ALTER TABLE public.fixture_table FORCE ROW LEVEL SECURITY"],
        ["pg_ts_config", "ALTER TEXT SEARCH CONFIGURATION public.fixture_config DROP MAPPING FOR asciiword"]
      ]
      cases.each do |catalog, mutation|
        mutation_test(catalog, setup, mutation)
      end
    end
  end
end

if $PROGRAM_NAME == __FILE__
  begin
    RevaerPostgresPristine::FocusedTests.new(File.expand_path("../..", __dir__)).run
  rescue RevaerPostgresPristine::Failure => error
    warn error.message
    exit 1
  end
end
