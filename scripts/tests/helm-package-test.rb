require 'digest'
require 'fileutils'
require 'json'
require 'open3'
require 'tmpdir'
require 'yaml'

$assertions = 0
def check(condition, label)
  $assertions += 1
  raise "Helm packaging regression: #{label}" unless condition
end

def run(env, *args)
  stdout, stderr, status = Open3.capture3(env, *args)
  check(status.success?, "#{args.join(' ')}: #{stdout}#{stderr}")
  check(!"#{stdout}#{stderr}".match?(/\bwarn(?:ing)?\b/i), 'no warnings')
  stdout
end

source = ARGV.fetch(0)
defaults = YAML.safe_load_file(File.join(source, 'charts/revaer/values.yaml'))
empty_fields = [%w[image digest], %w[image architecture], %w[image tag],
  %w[compliance existingClaim], %w[compliance manifestDigest]]
empty_fields.each { |path| check(defaults.dig(*path) == '', "empty source #{path.join('.')}") }
real_helm = run({}, 'just', '--command', 'bash', '-c', 'command -v helm').strip
version = '0.0.0-binding-test.0'
app_version = 'v0.0.0-binding-test.0'
uid = 'Fixture Chart Signer <signer@example.invalid>'
lint_args = ['--strict', '--set', 'database.url=postgres://db.example.test/revaer',
  '--set-string', "image.digest=sha256:#{'a' * 64}", '--set-string', 'image.architecture=amd64',
  '--set-string', 'image.tag=', '--set-string', 'compliance.existingClaim=lint-only-not-prepared',
  '--set-string', "compliance.manifestDigest=sha256:#{'b' * 64}"]

Dir.mktmpdir('revaer-helm-package-') do |temporary_root|
  root = File.realpath(temporary_root)
  workspace = File.join(root, 'workspace')
  FileUtils.mkdir_p(File.join(workspace, 'release/scripts'))
  FileUtils.mkdir_p(File.join(workspace, 'charts'))
  FileUtils.cp(File.join(source, 'justfile'), workspace)
  FileUtils.cp_r(File.join(source, 'just'), workspace)
  FileUtils.cp_r(File.join(source, 'charts/revaer'), File.join(workspace, 'charts'))
  %w[helm-package.sh render-helm-annotations.sh].each do |file|
    FileUtils.cp(File.join(source, 'release/scripts', file), File.join(workspace, 'release/scripts'))
  end
  bin = File.join(root, 'bin')
  temporary = File.join(root, 'temporary')
  FileUtils.mkdir_p([bin, temporary])

  # Only lint and unsigned packaging use real Helm. Signing is a command-boundary model.
  File.write(File.join(bin, 'helm'), <<~'HELM')
    #!/usr/bin/env ruby
    require 'json'
    require 'yaml'
    record = { 'tool' => 'helm', 'args' => ARGV }
    if ARGV.first == 'package'
      record['values'] = YAML.safe_load_file(File.join(ARGV.fetch(1), 'values.yaml'))
      if ARGV.include?('--keyring')
        record['keyring_mode'] = File.stat(ARGV.fetch(ARGV.index('--keyring') + 1)).mode & 0777
      end
    end
    File.open(ENV.fetch('FIXTURE_COMMAND_LOG'), 'a') { |file| file.puts(JSON.generate(record)) }
    exit 23 if ENV['FIXTURE_FAIL'] == ARGV.first
    if ENV.fetch('FIXTURE_MODE') == 'signing' && %w[package verify].include?(ARGV.first)
      exit 0
    end
    abort 'Unexpected Helm command in fixture' unless %w[lint package].include?(ARGV.first)
    exec ENV.fetch('FIXTURE_REAL_HELM'), *ARGV
  HELM
  File.write(File.join(bin, 'gpg'), <<~'GPG')
    #!/usr/bin/env ruby
    require 'json'
    abort 'Unsigned packaging must not invoke GPG' unless ENV.fetch('FIXTURE_MODE') == 'signing'
    File.open(ENV.fetch('FIXTURE_COMMAND_LOG'), 'a') do |file|
      file.puts(JSON.generate('tool' => 'gpg', 'args' => ARGV,
        'home_mode' => File.stat(ENV.fetch('GNUPGHOME')).mode & 0777))
    end
    case ARGV
    when %w[--batch --yes --import]
      STDIN.read
    when %w[--batch --export]
      puts 'command-construction fixture, not a public key'
    when %w[--batch --export-secret-keys]
      puts 'command-construction fixture, not a secret key'
    when %w[--batch --list-secret-keys --with-colons]
      puts 'uid' + ':' * 9 + 'Fixture Chart Signer <signer@example.invalid>:'
    when %w[--batch --list-secret-keys --with-colons --fingerprint]
      puts 'fpr' + ':' * 9 + '0123456789ABCDEF0123456789ABCDEF01234567:'
    else
      abort 'Unexpected GPG command in fixture'
    end
  GPG
  FileUtils.chmod(0755, [File.join(bin, 'helm'), File.join(bin, 'gpg')])
  cases = [['unsigned', 'unsigned', nil], ['unsigned-default-url', 'unsigned', nil], ['signing-commands', 'signing', nil],
    ['unsigned-lint-failure', 'unsigned', 'lint'], ['signing-lint-failure', 'signing', 'lint'],
    ['signing-package-failure', 'signing', 'package'], ['signing-verify-failure', 'signing', 'verify']]
  cases.each do |label, mode, failure|
    log = File.join(root, "#{label}.jsonl")
    env = {
      'PATH' => "#{bin}:#{ENV.fetch('PATH')}", 'TMPDIR' => temporary,
      'FIXTURE_REAL_HELM' => real_helm, 'FIXTURE_COMMAND_LOG' => log,
      'FIXTURE_MODE' => mode, 'FIXTURE_FAIL' => failure,
      'REVAER_HELM_SIGN' => mode == 'signing' ? '1' : '0',
      'REVAER_HELM_LINT_DATABASE_URL' => label == 'unsigned-default-url' ? nil : 'postgres://db.example.test/revaer',
      'REVAER_RELEASE_REPOSITORY' => 'fixture/revaer',
      'REVAER_HELM_IMAGE_REPOSITORY' => 'registry.example.test/team/revaer',
      'HELM_GPG_PRIVATE' => mode == 'signing' ? 'fixture, not a private key' : nil,
      'HELM_GPG_PUBLIC' => mode == 'signing' ? 'fixture, not a public key' : nil,
      'GNUPGHOME' => File.join(root, 'unused-key-home'),
      'ARTIFACTHUB_OWNER_NAME' => 'Fixture Owner',
      'ARTIFACTHUB_OWNER_EMAIL' => 'owner@example.invalid',
      'ARTIFACTHUB_REPOSITORY_ID' => nil
    }
    stdout, stderr, status = Open3.capture3(env, 'just', '--justfile', File.join(workspace, 'justfile'),
      '--working-directory', workspace, 'helm-package', version, app_version)
    check(failure ? !status.success? : status.success?, "#{label} status: #{stdout}#{stderr}")
    check("#{stdout}#{stderr}".include?('exit code 23'), "#{label} propagates failure") if failure
    check(!"#{stdout}#{stderr}".match?(/\bwarn(?:ing)?\b/i), "#{label} no warnings")
    commands = File.readlines(log).map { |line| JSON.parse(line) }
    helm_calls = commands.select { |entry| entry['tool'] == 'helm' }
    expected = failure == 'lint' ? ['lint'] : mode == 'signing' && failure != 'package' ? %w[lint package verify] : %w[lint package]
    check(helm_calls.map { |entry| entry.fetch('args').first } == expected, "#{label} exact command order")
    expected_lint = lint_args.dup
    expected_lint[2] = 'database.url=postgres://postgres.default.svc.cluster.local:5432/revaer' if label == 'unsigned-default-url'
    check(helm_calls.first.fetch('args').drop(2) == expected_lint, "#{label} strict explicit lint-only bindings")
    package = helm_calls.find { |entry| entry.dig('args', 0) == 'package' }
    dist = File.join(workspace, 'dist/helm')
    archive = File.join(dist, "revaer-#{version}.tgz")
    if package
      check(package.fetch('values') == defaults, "#{label} fixtures never enter chart values")
      expected_args = ['--destination', dist, '--version', version, '--app-version', app_version]
      if mode == 'signing'
        args = package.fetch('args')
        keyring = args.fetch(args.index('--keyring') + 1)
        expected_args += ['--sign', '--key', uid, '--keyring', keyring]
        check(package['keyring_mode'] == 0600, "#{label} private temporary keyring permissions")
      end
      check(package.fetch('args').drop(2) == expected_args, "#{label} exact package arguments")
    end
    if mode == 'signing'
      gpg_calls = commands.select { |entry| entry['tool'] == 'gpg' }
      check(gpg_calls.map { |entry| entry['args'] } == [
        %w[--batch --yes --import], %w[--batch --yes --import], %w[--batch --export],
        %w[--batch --export-secret-keys], %w[--batch --list-secret-keys --with-colons],
        %w[--batch --list-secret-keys --with-colons --fingerprint]
      ], "#{label} expected GPG boundary calls only")
      check(gpg_calls.all? { |entry| entry['home_mode'] == 0700 }, "#{label} private temporary key home")
      verification = helm_calls.find { |entry| entry.dig('args', 0) == 'verify' }
      if verification
        check(verification['args'] == ['verify', archive, '--keyring', File.join(dist, 'revaer-helm-public.gpg')], "#{label} exact verification arguments")
      end
      check(!File.exist?(archive), "#{label} model produces no purported signed artifact")
    else
      check(commands.none? { |entry| entry['tool'] == 'gpg' }, "#{label} no signing operations")
    end
    if mode == 'unsigned' && failure.nil?
      check(File.size(archive).positive?, 'real unsigned chart archive exists')
      packaged = YAML.safe_load(run({}, 'just', '--command', real_helm, 'show', 'values', archive))
      check(packaged == defaults, 'real packaged values equal source values')
      empty_fields.each { |path| check(packaged.dig(*path) == '', "empty packaged #{path.join('.')}") }
      metadata = YAML.safe_load(run({}, 'just', '--command', real_helm, 'show', 'chart', archive))
      check(metadata['version'] == version && metadata['appVersion'] == app_version, 'real packaged version arguments applied')
      puts "Unsigned fixture archive sha256:#{Digest::SHA256.file(archive).hexdigest}; empty bindings verified, not release qualification."
    end
    check(Dir.children(temporary).empty?, "#{label} temporary staging and key directories removed")
    puts "PASS #{label}#{mode == 'signing' ? ' (command-construction model only)' : ''}"
  end
end
puts "Helm packaging: 7 cases, #{$assertions} assertions; fixture archives and command models removed."
