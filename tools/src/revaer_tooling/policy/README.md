# Repository and workflow policy

These validators carry the media stack's existing quality and publication
contracts into Python. They return findings for the task layer to report; they
do not rewrite configuration, weaken gates, or change remote project settings.
Task wiring is implemented; workflow conversion is tracked in the
[migration inventory](../../../migration.md).

## Structure

| Module | Checks |
| --- | --- |
| `formats.py` | YAML syntax trees and Java properties without ambiguous keys, aliases, merges, or hidden overrides. |
| `advisories.py` | Empty advisory-ignore inputs; malformed or absent required policy fails. |
| `sonar.py` | Exact scanner property allowlist, complete authored source inventory, analyzer classification, and the empty test-scope sentinel. |
| `workflows.py` | Action pins, safe single-command steps, job dependencies, conditions, permissions, and bounded timeouts. |
| `contracts.py` | Shared structured comparisons for named jobs, command arguments, ordering, and artifacts. |
| `required_checks.py` | Required PR contexts, independent supply-chain results, database proof ordering, and complete E2E shards. |
| `scanner_workflows.py` | Same-checkout coverage and scan order, exact SCM event inputs, one scanner, and complete evidence retention. |
| `image_workflows.py` | Native architecture matrix, PR scans, publication predicates, immutable digests, compliance signing, and manifest/chart dependencies. |

Workflow YAML is read structurally. Quoted values and GitHub's `on` key retain
their literal meaning; duplicate keys cannot silently replace a required gate.
`run` steps dispatch a registered `rv` command. The setup composite has one
additional bootstrap allowance for `uv sync --locked`; it does not permit
arbitrary embedded shell or JavaScript.

## Entry points

`rv policy` composes source, advisory, Sonar and workflow validation. `rv lint`
runs that complete policy before Python lint/types and both Rust Clippy passes.
The CLI injects the actual command registry, so workflow commands must resolve
to a shipped task rather than a second hard-coded command list.

Direct `rv audit` and `rv deny` calls also validate the empty advisory policies.
Direct `rv sonar-scan` validates the exact properties and full source inventory
before invoking the scanner. It invalidates old scanner success evidence first,
including when configuration validation fails. The Sonar sentinel must be the
tracked, empty `.sonar-test-scope/.gitkeep`, with no extra directory entries.

## Maintaining a contract

1. Read the root and scoped instructions and the affected workflow together.
2. Update the Python task and its typed external-tool arguments.
3. Update the workflow and the smallest applicable structural contract.
4. Add a behavior regression showing the unsafe or incomplete change fails.
5. Run the relevant policy tests and actual task checks. Complete the repository
   and integration gates before removing the former implementation.

Tests use `tools/tests/fixtures/workflow-contracts.json`, derived from media
revision `f1a8624f8c9c80325bfd14769bc1305c0fa5b793`. Adaptations replace Just commands
with `rv`, replace retired release-JavaScript coverage with Python tooling
coverage, retain scanner steps in their producing job, and add Python evidence.
The fixture records the source revision. It is a semantic contract fixture;
remaining legacy setup blocks in it do not claim compliance with the new
single-command workflow syntax. The final migrated workflows must satisfy both
the syntax validator and their semantic contracts.

The source snapshot is not the active media worktree. Refresh ongoing work before
integrated validation, and preserve its newly added checks and scenario needs.

## Media stack regression coverage

The workflow guard within `rv policy` includes the media conversion contract:
canonical conversion execution, a required report uploaded even after failure,
unconditional fixture cleanup after upload, and downstream image/release jobs
that depend on conversion. Release builds cannot have a pull-request-skipping
job condition. The supply-chain aggregate still requires all three upstream
results under `always()`.

`rv tooling-check` runs independent mutations for these conditions in
`test_workflow_contracts.py`; it replaces the old standalone stack-contract test
recipe. The original advisory exception cases also run there through
`test_policy_tasks.py`, exercising policy, audit and deny entry points. No
separate public command is needed for these internal regression suites.
