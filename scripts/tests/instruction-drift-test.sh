#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

exec ruby --disable-gems - "${repo_root}" <<'RUBY'
require 'fileutils'
require 'open3'
require 'tmpdir'

$assertions = 0
$guard_runs = 0
def check(condition, label)
  $assertions += 1
  raise "Instruction drift regression: #{label}" unless condition
end

def command(root, *args, env: {})
  Open3.capture3(env, 'just', '--justfile', File.join(root, 'justfile'),
    '--working-directory', root, *args)
end

def git(root, *args)
  output, error, status = command(root, '--command', 'git', *args)
  check(status.success?, "fixture git #{args.first}: #{output}#{error}")
  output.strip
end

def edit(root, path)
  target = File.join(root, path)
  FileUtils.mkdir_p(File.dirname(target))
  File.open(target, 'a') { |file| file.puts("\n# fixture-only change") }
end

def evaluate(root, base, path, accepted)
  $guard_runs += 1
  output, error, status = command(root, 'instruction-drift', env: {
    'REVAER_INSTRUCTION_DIFF_BASE' => base, 'REVAER_INSTRUCTION_DIFF_HEAD' => 'HEAD'
  })
  check(status.success? == accepted, "#{path} expected #{accepted ? 'acceptance' : 'DevOps rejection'}: #{output}#{error}")
  return if accepted

  check(error.include?('Changed workflow/release files require an update to AGENTS.md or .github/instructions/devops.instructions.md:'), 'DevOps diagnostic, not an unrelated failure')
  check(error.lines.any? { |line| line.strip == path }, 'changed target retained in diagnostic')
end

source = ARGV.fetch(0)
paths = %w[charts/revaer/values.yaml charts/revaer/templates/nested/probe.yaml
  charts/revaer/README.md scripts/tests/compliance-chart-test.sh
  scripts/tests/helm-package-test.sh scripts/tests/instruction-drift-test.sh
  scripts/instruction-drift-check.sh just/release.just release/scripts/helm-package.sh]
outside = %w[docs/tasks/fixture.md scripts/tests/compliance-chart-test.sh.backup]

Dir.mktmpdir('revaer-instruction-drift-') do |temporary|
  (paths + outside).each_with_index do |path, index|
    root = File.join(temporary, "case-#{index}")
    FileUtils.mkdir_p(File.join(root, 'scripts'))
    FileUtils.mkdir_p(File.join(root, '.github/instructions'))
    FileUtils.cp(File.join(source, 'justfile'), root)
    FileUtils.cp_r(File.join(source, 'just'), root)
    FileUtils.cp(File.join(source, 'scripts/instruction-drift-check.sh'), File.join(root, 'scripts'))
    %w[devops.instructions.md rust.instructions.md].each do |file|
      File.write(File.join(root, '.github/instructions', file), "# Fixture instructions\n")
    end
    git(root, 'init', '--quiet', '--initial-branch=fixture')
    git(root, 'config', 'user.name', 'Instruction Fixture')
    git(root, 'config', 'user.email', 'fixture@example.invalid')
    git(root, 'config', 'commit.gpgsign', 'false')
    git(root, 'config', 'core.hooksPath', '/dev/null')
    git(root, 'add', '.')
    git(root, 'commit', '--quiet', '-m', 'test: seed isolated instruction fixture')
    base = git(root, 'rev-parse', 'HEAD')
    edit(root, path)
    evaluate(root, '', path, false) if index.zero?
    git(root, 'add', '--', path)
    evaluate(root, '', path, false) if index.zero?
    git(root, 'commit', '--quiet', '-m', 'test: change isolated target')
    if outside.include?(path)
      evaluate(root, base, path, true)
      puts "PASS #{path} is outside the new DevOps mapping"
      next
    end
    evaluate(root, base, path, false)
    edit(root, '.github/instructions/rust.instructions.md')
    git(root, 'add', '.github/instructions/rust.instructions.md')
    git(root, 'commit', '--quiet', '-m', 'test: unrelated instruction update')
    evaluate(root, base, path, false)
    edit(root, '.github/instructions/devops.instructions.md')
    git(root, 'add', '.github/instructions/devops.instructions.md')
    git(root, 'commit', '--quiet', '-m', 'test: matching instruction update')
    evaluate(root, base, path, true)
    puts "PASS #{path} requires the DevOps instruction update"
  end
end
puts "Instruction drift: #{paths.length + outside.length} paths, #{$guard_runs} guard runs, #{$assertions} assertions; isolated Git fixtures removed."
RUBY
