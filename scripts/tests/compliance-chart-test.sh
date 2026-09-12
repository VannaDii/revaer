#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

exec ruby --disable-gems - "${repo_root}" <<'RUBY'
require 'fileutils'
require 'json'
require 'open3'
require 'tmpdir'
require 'yaml'

$assertions = 0
$helm_calls = 0

def check(condition, label)
  $assertions += 1
  raise "Compliance chart regression: #{label}" unless condition
end

def documents(source)
  stream = Psych.parse_stream(source)
  visit = lambda do |node|
    if node.is_a?(Psych::Nodes::Mapping)
      keys = node.children.each_slice(2).map do |key, _value|
        check(key.is_a?(Psych::Nodes::Scalar), 'scalar YAML key')
        key.value
      end
      check(keys.uniq == keys, 'no duplicate YAML mapping keys')
    end
    node.children&.each { |child| visit.call(child) }
  end
  visit.call(stream)
  stream.children.filter_map do |doc|
    single = Psych::Nodes::Stream.new
    single.children << doc
    YAML.safe_load(single.to_yaml, aliases: false)
  end
end

def helm(*args)
  $helm_calls += 1
  Open3.capture3('just', '--command', 'helm', *args)
end

def accepted(*args)
  stdout, stderr, status = helm(*args)
  check(status.success?, "#{args.first} succeeds: #{stdout}#{stderr}")
  check(!"#{stdout}#{stderr}".match?(/\bwarn(?:ing)?\b/i), 'no Helm warnings')
  stdout
end

def mutated(values, path, value)
  copy = JSON.parse(JSON.generate(values))
  copy.dig(*path[0...-1])[path.last] = value
  copy
end

def binding(rendered, values)
  deployments = rendered.select { |doc| doc['kind'] == 'Deployment' }
  check(deployments.length == 1, 'one Deployment')
  template = deployments.first.dig('spec', 'template')
  spec = template.fetch('spec')
  containers = spec.fetch('containers')
  check(containers.length == 1 && !spec.key?('initContainers'), 'no runtime installer or sidecar')
  app = containers.first
  check(app['image'] == "#{values.dig('image', 'repository')}@#{values.dig('image', 'digest')}", 'exact digest image helper')
  check(spec['nodeSelector'] == values['nodeSelector'].merge('kubernetes.io/arch' => values.dig('image', 'architecture')), 'architecture plus unrelated selectors')
  volumes = spec.fetch('volumes').select { |volume| volume['name'] == 'compliance' }
  check(volumes == [{ 'name' => 'compliance', 'persistentVolumeClaim' => {
    'claimName' => values.dig('compliance', 'existingClaim'), 'readOnly' => true
  } }], 'only an existing read-only compliance PVC')
  mounts = app.fetch('volumeMounts').select { |mount| mount['name'] == 'compliance' }
  image_hex = values.dig('image', 'digest').delete_prefix('sha256:')
  manifest_hex = values.dig('compliance', 'manifestDigest').delete_prefix('sha256:')
  check(mounts == [{ 'name' => 'compliance', 'mountPath' => '/app/compliance',
    'subPath' => "#{image_hex}/#{manifest_hex}", 'readOnly' => true }], 'exact read-only immutable subPath')
  annotations = template.dig('metadata', 'annotations')
  check(annotations['checksum/compliance-manifest'] == values.dig('compliance', 'manifestDigest'), 'exact manifest checksum')
  check(values['podAnnotations'].all? { |key, value| annotations[key] == value }, 'unrelated annotations preserved')
  check(app['imagePullPolicy'] == values.dig('image', 'pullPolicy'), 'pull policy retained')
  check(app['resources'] == values['resources'], 'resources retained')
  check(app['securityContext'] == values['containerSecurityContext'], 'container security retained')
  check(spec['securityContext'] == values['podSecurityContext'], 'pod security retained')
  check(%w[readinessProbe livenessProbe startupProbe].all? { |key| app.dig(key, 'exec', 'command')&.last == 'curl -fsS http://127.0.0.1:7070/health >/dev/null' }, 'health probes retained')
  check(app.fetch('env').none? { |entry| entry['name'].start_with?('REVAER_COMPLIANCE', 'REVAER_EXEC_') }, 'no E1 environment activation')
  check(rendered.none? { |doc| doc['kind'] == 'PersistentVolumeClaim' && doc.dig('metadata', 'name') == values.dig('compliance', 'existingClaim') }, 'no compliance PVC provisioned')
  deployments.first
end

chart = File.join(ARGV.fetch(0), 'charts/revaer')
base = YAML.safe_load_file(File.join(chart, 'values.yaml'))
# Synthetic renderer inputs only: these hashes identify no release or prepared storage.
base['image'].merge!('digest' => "sha256:#{'a' * 64}", 'architecture' => 'amd64')
base['compliance'].merge!('existingClaim' => 'prepared-compliance', 'manifestDigest' => "sha256:#{'b' * 64}")
base['database']['existingSecret'] = 'fixture-db'

valid = []
%w[amd64 arm64].each do |arch|
  values = mutated(base, %w[image architecture], arch)
  valid << ["#{arch}-minimal", values]
  values = mutated(values, %w[nodeSelector kubernetes.io/arch], arch)
  values['nodeSelector']['storage.example.test/class'] = 'fast'
  values['podAnnotations']['example.test/note'] = 'preserved'
  valid << ["#{arch}-matching-selectors", values]
end
valid << ['image-rollover', mutated(base, %w[image digest], "sha256:#{'c' * 64}")]
valid << ['manifest-rollover', mutated(base, %w[compliance manifestDigest], "sha256:#{'d' * 64}")]
valid << ['unqualified-repository', mutated(base, %w[image repository], 'revaer')]
valid << ['minimum-claim', mutated(base, %w[compliance existingClaim], 'a')]
valid << ['maximum-claim', mutated(base, %w[compliance existingClaim], "#{'a' * 63}.#{'b' * 63}.#{'c' * 63}.#{'d' * 61}")]
custom = mutated(base, %w[image repository], 'registry.example.test:5000/team/revaer')
custom['image']['pullPolicy'] = 'Always'
custom['imagePullSecrets'] = [{ 'name' => 'fixture-registry' }]
custom['database'].merge!('existingSecret' => '', 'url' => 'postgres://db.example.test/revaer')
custom['configPersistence'].merge!('enabled' => true, 'existingClaim' => 'existing-config')
custom['dataPersistence']['enabled'] = true
custom['serviceAccount'].merge!('create' => false, 'name' => 'existing-account')
custom['ingress']['enabled'] = true
custom['extraEnv'] = [{ 'name' => 'EXAMPLE_SETTING', 'value' => 'retained' }]
custom['extraEnvFrom'] = [{ 'configMapRef' => { 'name' => 'fixture-env' } }]
custom['podLabels'] = { 'example.test/label' => 'retained' }
custom['resources'] = { 'requests' => { 'cpu' => '100m' } }
custom['podSecurityContext'] = { 'runAsNonRoot' => true }
custom['tolerations'] = [{ 'key' => 'fixture', 'operator' => 'Exists' }]
custom['affinity'] = { 'podAffinity' => { 'preferredDuringSchedulingIgnoredDuringExecution' => [] } }
valid << ['unrelated-chart-behavior', custom]

invalid = [['unconfigured-defaults', YAML.safe_load_file(File.join(chart, 'values.yaml')), /image|compliance/, true]]
bad_digests = ['', nil, 'latest', 'a' * 64, "sha256:#{'a' * 63}", "sha256:#{'a' * 65}",
  "sha256:#{'A' * 64}", "SHA256:#{'a' * 64}", "sha256:#{'g' * 64}", "sha512:#{'a' * 128}",
  " sha256:#{'a' * 64}", "sha256:#{'a' * 64}\n", "sha256:#{'a' * 64}/../other", true, 42, []]
[%w[image digest], %w[compliance manifestDigest]].each do |path|
  bad_digests.each_with_index do |value, index|
    invalid << ["#{path.join('-')}-#{index}", mutated(base, path, value), /#{path.join('.*')}/, true]
  end
end
{
  %w[image repository] => ['', nil, true, 42, [], 'revaer:latest', 'revaer:5000',
    'registry.example.test/team/revaer:mutable', 'registry.example.test:5000/team/revaer:mutable',
    "registry.example.test/team/revaer@sha256:#{'c' * 64}", 'registry.example.test:tag/team/revaer',
    'registry.example.test/team:tag/revaer', 'https://registry.example.test/team/revaer',
    'oci://registry.example.test/team/revaer', ' revaer', 'revaer ', "revaer\n", "revaer\t",
    "registry.example.test/\u00a0revaer", "revaer\u0000"],
  %w[image architecture] => ['', nil, 'x86_64', 'aarch64', 'AMD64', "arm64\n", true, []],
  %w[image tag] => ['latest', 'v1.2.3', ' ', "\n", nil, 42, false],
  %w[compliance existingClaim] => ['', nil, ' ', ' prepared', 'bad/name', 'bad..name', 'Upper',
    'bad_name', '-leading', 'trailing-', 'x' * 254, true, 42, []]
}.each do |path, rejected|
  rejected.each_with_index do |value, index|
    invalid << ["#{path.join('-')}-#{index}", mutated(base, path, value), /#{path.join('.*')}/, true]
  end
end
%w[amd64 arm64].each do |arch|
  values = mutated(base, %w[image architecture], arch)
  values['nodeSelector']['kubernetes.io/arch'] = arch == 'amd64' ? 'arm64' : 'amd64'
  invalid << ["#{arch}-conflicting-selector", values, /nodeSelector/, true]
end
['tampered', '', base.dig('compliance', 'manifestDigest'), nil].each_with_index do |value, index|
  invalid << ["reserved-annotation-#{index}", mutated(base, ['podAnnotations', 'checksum/compliance-manifest'], value), /podAnnotations/, true]
end
invalid << ['unsupported-compliance-toggle', mutated(base, %w[compliance enabled], false), /compliance/, false]
invalid << ['unsupported-subpath-override', mutated(base, %w[compliance subPath], 'latest'), /compliance/, false]

Dir.mktmpdir('revaer-compliance-chart-') do |root|
  schema_chart = File.join(root, 'schema-only')
  FileUtils.mkdir(schema_chart)
  %w[Chart.yaml values.yaml values.schema.json].each { |file| FileUtils.cp(File.join(chart, file), schema_chart) }
  values_file = File.join(root, 'values.json')
  results = {}
  valid.each do |label, values|
    File.write(values_file, JSON.generate(values))
    accepted('lint', chart, '--strict', '-f', values_file)
    rendered = documents(accepted('template', 'fixture', chart, '-f', values_file))
    results[label] = binding(rendered, values)
    check(accepted('template', 'fixture', schema_chart, '-f', values_file).strip.empty?, 'independent schema acceptance')
    check(rendered == documents(accepted('template', 'fixture', chart, '-f', values_file, '--skip-schema-validation')), 'template-only acceptance unchanged')
    if label == 'unrelated-chart-behavior'
      spec = results[label].dig('spec', 'template', 'spec')
      app = spec.fetch('containers').first
      check(spec['imagePullSecrets'] == values['imagePullSecrets'], 'registry secrets retained')
      check(spec['serviceAccountName'] == 'existing-account', 'existing service account retained')
      check(spec['affinity'] == values['affinity'] && spec['tolerations'] == values['tolerations'], 'affinity and tolerations retained')
      check(results[label].dig('spec', 'template', 'metadata', 'labels')['example.test/label'] == 'retained', 'pod labels retained')
      check(app.fetch('env').include?(values['extraEnv'].first) && app['envFrom'] == values['extraEnvFrom'], 'extra environment retained')
      check(spec.fetch('volumes').find { |volume| volume['name'] == 'config' }['persistentVolumeClaim'] == { 'claimName' => 'existing-config' }, 'config claim retained')
      check(app.fetch('volumeMounts').select { |mount| %w[config data].include?(mount['name']) } == [{ 'name' => 'config', 'mountPath' => '/config' }, { 'name' => 'data', 'mountPath' => '/data' }], 'config and data mounts retained')
      check(rendered.count { |doc| doc['kind'] == 'PersistentVolumeClaim' } == 1, 'data PVC still provisioned')
      check(%w[Ingress Secret Service].all? { |kind| rendered.any? { |doc| doc['kind'] == kind } }, 'other resources retained')
      check(results[label].dig('spec', 'template', 'metadata', 'annotations')['checksum/database-secret']&.match?(/\A[a-f0-9]{64}\z/), 'database checksum retained')
    end
    puts "PASS #{label}"
  end
  invalid.each do |label, values, diagnostic, template_guard|
    File.write(values_file, JSON.generate(values))
    modes = [[chart, []], [schema_chart, []]]
    modes << [chart, ['--skip-schema-validation']] if template_guard
    modes.each do |target, flags|
      stdout, stderr, status = helm('template', 'fixture', target, '-f', values_file, *flags)
      check(!status.success?, "#{label} rejected by #{File.basename(target)} #{flags}")
      check("#{stdout}#{stderr}".match?(diagnostic), "#{label} failed for the intended field: #{stdout}#{stderr}")
      check(stdout.strip.empty?, "#{label} emits no installable manifest")
    end
    puts "PASS #{label} rejected"
  end
end
puts "Compliance chart: #{valid.length + invalid.length} cases (#{valid.length} accepted, #{invalid.length} rejected), #{$helm_calls} Helm calls, #{$assertions} assertions; temporary fixtures removed."
RUBY
