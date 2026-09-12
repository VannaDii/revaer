require 'fileutils'
require 'json'
require 'open3'
require 'tmpdir'

$assertions = 0
$runs = 0

def check(condition, label)
  $assertions += 1
  raise "Test database input regression: #{label}" unless condition
end

def consumer?(entry)
  arguments = entry.fetch('arguments')
  (entry.fetch('tool') == 'just' && arguments.first == 'db-start') ||
    (entry.fetch('tool') == 'cargo' && (arguments.include?('test') || arguments.include?('--no-report')))
end

source = File.realpath(ARGV.fetch(0))
just_path = ENV.fetch('PATH').split(File::PATH_SEPARATOR).map { |path| File.join(path, 'just') }
  .find { |path| File.file?(path) && File.executable?(path) }
raise 'Just executable not found' unless just_path

recipes = { 'test' => 1, 'test-native' => 1, 'test-features-min' => 2, 'validate' => 1, 'cov' => 2 }
test_url = 'postgres://test-db.example.invalid/test_fixture'
database_url = 'postgres://app-db.example.invalid/app_fixture'
cases = [
  [nil, nil, nil], ['', '', nil], ['', nil, nil], [nil, '', nil],
  [test_url, nil, [test_url, test_url]],
  [nil, database_url, [database_url, database_url]],
  [test_url, database_url, [test_url, database_url]],
  ['', database_url, [database_url, database_url]],
  [test_url, '', [test_url, test_url]]
]

Dir.mktmpdir('revaer-test-database-inputs-') do |root|
  FileUtils.cp(File.join(source, 'justfile'), root)
  FileUtils.cp_r(File.join(source, 'just'), root)
  FileUtils.mkdir_p(File.join(root, 'scripts'))
  FileUtils.mkdir_p(File.join(root, 'bin'))
  File.write(File.join(root, 'Cargo.toml'), "[workspace]\nmembers = [\n]\n")
  %w[ensure-exact-cargo-tool.sh coverage-toolchain-env.sh].each do |helper|
    File.write(File.join(root, 'scripts', helper), "#!/usr/bin/env bash\nset -euo pipefail\n")
  end
  # Exercise the real recipes without starting a database, installing tools or compiling.
  %w[cargo just rustup].each do |tool|
    path = File.join(root, 'bin', tool)
    File.write(path, <<~'RUBY')
      #!/usr/bin/env ruby
      require 'json'
      tool = File.basename($PROGRAM_NAME)
      entry = { tool: tool, arguments: ARGV,
        test_url: ENV['REVAER_TEST_DATABASE_URL'], database_url: ENV['DATABASE_URL'] }
      File.open(ENV.fetch('REVAER_RECIPE_CALLS'), 'a') { |file| file.puts(JSON.generate(entry)) }
      consumer = (tool == 'just' && ARGV.first == 'db-start') ||
        (tool == 'cargo' && (ARGV.include?('test') || ARGV.include?('--no-report')))
      exit 17 if consumer && ENV.fetch('REVAER_RECIPE_FAIL') == '1'
    RUBY
    FileUtils.chmod(0o700, path)
  end

  run = lambda do |recipe, supplied_test, supplied_database, fail_command|
    $runs += 1
    calls = File.join(root, "calls-#{$runs}.jsonl")
    environment = {
      'PATH' => "#{File.join(root, 'bin')}#{File::PATH_SEPARATOR}#{ENV.fetch('PATH')}",
      'REVAER_TEST_DATABASE_URL' => supplied_test, 'DATABASE_URL' => supplied_database,
      'REVAER_RECIPE_CALLS' => calls, 'REVAER_RECIPE_FAIL' => fail_command ? '1' : '0',
      'REVAER_DB_MANAGED' => '1'
    }
    _output, error, status = Open3.capture3(environment, just_path, '--no-dotenv',
      '--justfile', File.join(root, 'justfile'), recipe, chdir: root)
    entries = File.exist?(calls) ? File.readlines(calls).map { |line| JSON.parse(line) } : []
    [status, error, entries.select { |entry| consumer?(entry) }]
  end

  recipes.each do |recipe, consumer_count|
    cases.each do |supplied_test, supplied_database, expected|
      status, error, consumers = run.call(recipe, supplied_test, supplied_database, false)
      check(status.success? == !expected.nil?, "#{recipe} required explicit database input")
      if expected.nil?
        check(consumers.empty?, "#{recipe} launched database-backed work without input")
        check(error.include?('caller-supplied disposable test database'), "#{recipe} actionable missing-input diagnostic")
        next
      end

      check(consumers.length == consumer_count, "#{recipe} executed every database-backed command")
      consumers.each do |entry|
        check([entry.fetch('test_url'), entry.fetch('database_url')] == expected,
          "#{recipe} propagated the explicit URL precedence")
        if recipe == 'test-native' && entry.fetch('tool') == 'cargo'
          check(entry.fetch('arguments').include?('--test-threads=1'), 'native tests retained serialized execution')
        end
      end
    end
    status, _error, consumers = run.call(recipe, test_url, database_url, true)
    check(!status.success?, "#{recipe} propagated the database-backed command failure")
    check(consumers.length == 1, "#{recipe} stopped after the first database-backed failure")
  end
end

puts "Test database inputs: #{$runs} recipe runs, #{$assertions} assertions; temporary fixtures removed."
